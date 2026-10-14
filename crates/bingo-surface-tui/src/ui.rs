//! Everything that is the surface's own and nothing that is the session's.
//! The transcript, the turn, the queue and the open interactions all live in
//! `SessionState`; what lives here is what a second client would not share —
//! where the caret is, what is scrolled, what was armed and when.
//!
//! Every time-dependent decision takes `now`, so a test never sleeps.

use std::time::{Duration, Instant};

use bingo_sdk::{CommandSpec, Level, SessionId, SessionSummary, View};

use crate::clock::{Anim, FRAME, Now};
use crate::commands::{self, Suggestion};
use crate::complete;
use crate::fold::Folds;
use crate::graphics::{Decoded, Linked};
use crate::layers::{self, Reveal};
use crate::layout::Layout;
use crate::pager::Pager;
use crate::rewind::Rewind;
use crate::roster;
use crate::scroll::Scroll;
use crate::select::{self, Cell, Dragging, Edge};
use crate::tree::{self, Row, Tree};

mod controls;

// Keep callers at the UI boundary while input state has one owner.
impl std::ops::Deref for Ui {
    type Target = controls::Controls;
    fn deref(&self) -> &Self::Target {
        &self.controls
    }
}
impl std::ops::DerefMut for Ui {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.controls
    }
}

/// How long a transient notice holds the status line's middle slot (§3),
/// between the frames it fades in over and the ones it fades out over.
pub const NOTICE: Duration = Duration::from_secs(4);
/// A notice fades into the slot over two frames, and out of it over two (§6).
pub const NOTICE_FADE: Duration = Duration::from_millis(2 * FRAME.as_millis() as u64);
/// A second ctrl+c within this window leaves.
pub const EXIT_WINDOW: Duration = Duration::from_secs(2);
/// A transcript that has just been stepped into crossfades through dim over
/// this long: two frames (§6).
pub const SWITCH: Duration = Duration::from_millis(2 * FRAME.as_millis() as u64);
/// One light along the input box's border when a line is sent: six frames
/// (§6). The most repeated gesture in the surface, and the only motion that
/// answers a person rather than reporting the model.
pub const IGNITION: Duration = Duration::from_millis(6 * FRAME.as_millis() as u64);

/// A message that is not transcript: an ack, a warning, a hint.
#[derive(Clone, Debug)]
pub struct Notice {
    pub level: Level,
    pub text: String,
    /// What it is about, said after it in dim: the line a refused intent
    /// carried, so a person sees which of theirs came back.
    pub about: Option<String>,
    /// When it took the slot — one notice is read at a time, and the next
    /// waits (§3). `None` until its turn comes.
    shown: Option<Instant>,
}

impl Notice {
    /// How strongly it is being said: 0 as it arrives, 1 while it is there to
    /// be read, 0 again as it goes — and nothing at all while it waits.
    pub fn strength(&self, now: Now) -> Option<f32> {
        let shown = self.shown?;
        if !now.motion {
            return Some(1.0);
        }
        let held = NOTICE_FADE + NOTICE;
        match now.since(shown) {
            age if age < NOTICE_FADE => Some(Anim::new(shown, NOTICE_FADE).progress(now.instant)),
            age if age < held => Some(1.0),
            _ => Some(1.0 - Anim::new(shown + held, NOTICE_FADE).progress(now.instant)),
        }
    }

    pub fn next_change(&self, now: Now) -> Option<Instant> {
        let shown = self.shown?;
        let expiry = shown + self.life();
        let changes = [
            shown + NOTICE_FADE,
            shown + NOTICE_FADE + NOTICE + Duration::from_nanos(1),
            expiry,
        ];
        if now.motion {
            changes.into_iter().find(|at| *at > now.instant)
        } else {
            (expiry > now.instant).then_some(expiry)
        }
    }

    /// Whether it has said its piece and left.
    fn gone(&self, now: Instant) -> bool {
        self.shown
            .is_some_and(|shown| now.saturating_duration_since(shown) >= self.life())
    }

    fn life(&self) -> Duration {
        NOTICE_FADE + NOTICE + NOTICE_FADE
    }
}

/// The command dropdown's own state; its rows are derived from the composer.
#[derive(Clone, Copy, Debug, Default)]
pub struct Menu {
    pub selected: usize,
    /// Esc closed it; typing opens it again.
    pub dismissed: bool,
}

/// The `/resume` picker, filled by the host and answered by Enter.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Picker {
    pub sessions: Vec<SessionSummary>,
    pub selected: usize,
}

/// The list of sessions as it was drawn: where its rows are, and which row of
/// the list each drawn line is — what a click on it needs to know. A memo of
/// the draw, like [`Card`].
#[derive(Clone, Debug, Default)]
pub struct Listed {
    pub area: ratatui::layout::Rect,
    pub roster: roster::Roster,
}

impl Listed {
    /// The row a pointer is over: which line of the list it landed on, and the
    /// cursor that line answers to. A label and a `…` mark answer nobody.
    pub fn at(&self, row: u16) -> Option<roster::Cursor> {
        let line = row.checked_sub(self.area.y)?;
        self.roster.rows.get(usize::from(line)).copied().flatten()
    }
}

/// A card as it was drawn: where its box is, and which option each of its
/// rows belongs to — what a click on it needs to know.
#[derive(Clone, Debug, Default)]
pub struct Card {
    pub area: ratatui::layout::Rect,
    pub options: Vec<Option<usize>>,
    pub sealed_options: Vec<(ratatui::layout::Rect, usize)>,
}

/// What the kernel told this surface it can offer: the commands a session
/// runs, and the ids each of their catalogued arguments may take. Read once
/// at start — the dropdown ranks them, it does not watch them.
#[derive(Clone, Debug, Default)]
pub struct Catalogs {
    pub commands: Vec<CommandSpec>,
    pub values: commands::Catalogues,
}

/// The one list of sessions, opened by `↓` on an empty composer or by
/// `ctrl+g`. Its rows are derived at render time; what is the surface's own is
/// where the cursor is, the line the box was holding when it opened, the one
/// listing the host answered with, and where the gesture started — nothing
/// here watches the store.
///
/// The query is **not** here (M58): what narrows the list is what is in the
/// input box, as it is under the `/` and `@` dropdowns.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Switcher {
    pub cursor: roster::Cursor,
    /// The line a person was writing when the list went up, set aside while
    /// the box is the query line and put back exactly as it was when the list
    /// goes. A fact about the gesture, not a second composer.
    pub draft: String,
    /// Empty until the read the opening spawned lands.
    pub stored: Vec<SessionSummary>,
    /// The session the list was opened from: what `esc` gives back after a
    /// walk has switched the view. A fact about the gesture, like
    /// [`Ui::esc_armed`] — not a second copy of what the tree is showing.
    pub from: Option<SessionId>,
}

impl Switcher {
    /// The session a row of this list names. The list is composed here from
    /// the tree, the store's answer and the query the box holds, so a click
    /// and a keypress cannot be reading two different lists.
    pub fn session(&self, tree: &Tree, query: &str, cursor: roster::Cursor) -> Option<SessionId> {
        self.of(tree, query, cursor, |row| row.session.clone())
    }

    /// What that row is called — a session's name, a room's `#name`: what
    /// `tab` completes into the box.
    pub fn name(&self, tree: &Tree, query: &str, cursor: roster::Cursor) -> Option<String> {
        self.of(tree, query, cursor, |row| row.name.clone())
    }

    /// One walk of the list, and whatever the caller wants off the row it
    /// lands on.
    fn of<T>(
        &self,
        tree: &Tree,
        query: &str,
        cursor: roster::Cursor,
        want: impl Fn(&Row<'_>) -> T,
    ) -> Option<T> {
        let rows = tree::roster(tree, &self.stored);
        cursor.row(&roster::listing(tree, &rows, query)).map(want)
    }
}

/// What is over the frame. One at a time: focus moves into a layer and back
/// out, never sideways (§7).
#[derive(Clone, Debug, Default, PartialEq)]
pub enum Open {
    #[default]
    Nothing,
    /// The binding table and the commands, as a sheet.
    Help,
    /// The `/resume` list, as a sheet.
    Picker(Picker),
    /// One block, whole, as a sheet.
    Pager(Pager),
    /// The turns of this transcript, as a card above the input box.
    Rewind(Rewind),
    /// The tree, as a card above the input box.
    Switcher(Switcher),
}

impl Open {
    /// Whether the keyboard belongs to it while it is open. The lists and the
    /// panel answer keys; help is read while a person goes on typing.
    pub fn captures(&self) -> bool {
        matches!(
            self,
            Open::Picker(_) | Open::Pager(_) | Open::Switcher(_) | Open::Rewind(_)
        )
    }

    /// How many frames its arrival takes: a card comes down, a sheet rises.
    fn frames(&self) -> u16 {
        match self {
            Open::Switcher(_) | Open::Rewind(_) => layers::CARD_FRAMES,
            _ => layers::SHEET_FRAMES,
        }
    }
}

/// What is open and how far in it is. Closing runs the arrival backwards, so
/// what is going stays on the screen until it has gone.
#[derive(Clone, Debug)]
pub struct Layer {
    pub open: Open,
    since: Instant,
    closing: bool,
}

impl Layer {
    pub(crate) fn shut(now: Instant) -> Self {
        Self {
            open: Open::Nothing,
            since: now,
            closing: false,
        }
    }

    /// How far in it is at this instant. Where nothing may move, a layer is
    /// whole the moment it opens and gone the moment it closes.
    pub fn reveal(&self, now: Now) -> Reveal {
        if !now.motion {
            return match self.closing {
                true => Reveal::none(self.open.frames()),
                false => Reveal::whole(self.open.frames()),
            };
        }
        Reveal::at(self.open.frames(), self.since, now.instant, self.closing)
    }

    /// How far in it is, or nothing at all when there is nothing over the
    /// frame — including the moment after the last frame of a leaving.
    pub fn drawn(&self, now: Now) -> Option<Reveal> {
        let reveal = self.reveal(now);
        (self.open != Open::Nothing && !reveal.gone()).then_some(reveal)
    }

    pub fn next_change(&self, now: Now) -> Option<Instant> {
        if self.open == Open::Nothing || !self.reveal(now).moving() {
            return None;
        }
        let elapsed = now.since(self.since).as_nanos() / FRAME.as_nanos();
        Some(self.since + FRAME * u32::try_from(elapsed + 1).unwrap_or(u32::MAX))
    }

    /// Open this one, from the first frame.
    pub fn show(&mut self, open: Open, now: Instant) {
        self.open = open;
        self.since = now;
        self.closing = false;
    }

    /// Start closing whatever is open; [`Ui::expire`] takes it away once the
    /// last frame of its leaving has been drawn.
    pub fn close(&mut self, now: Instant) {
        if self.open == Open::Nothing || self.closing {
            return;
        }
        self.since = now;
        self.closing = true;
    }

    pub fn showing(&self) -> bool {
        self.open != Open::Nothing && !self.closing
    }
}

#[derive(Debug, Default)]
pub struct Placed {
    pub panes: crate::pane::State,
    pub pulled: crate::pulled::Memo,
    pub toasts: crate::toast::State,
}

#[derive(Debug)]
pub struct Ui {
    pub placed: Placed,
    controls: controls::Controls,
    pub scroll: Scroll,
    pub notices: Vec<Notice>,
    /// A command's `View`, shown until the next key.
    pub block: Option<View>,
    pub embers: crate::embers::Embers,
    /// How much of each block a person opened or shut, with `ctrl+o` or a
    /// click (§7). A block that is not in it wears its kind's own start
    /// ([`crate::fold`]), so the default is never written down twice.
    pub folds: Folds,
    /// Whether the terminal window is being looked at. A window nobody is
    /// looking at is the one that may say something to the desktop (§6).
    pub focused: bool,
    /// When the session in view was last stepped into: what the transcript
    /// crossfades from.
    pub switched: Option<Instant>,
    /// When a line was last sent: what the box's border sweeps from. A fact
    /// about the gesture, like [`Ui::switched`] and [`Ui::stop_asked`] — the
    /// session's own stream says what became of the line, and this says only
    /// that a person pressed the key.
    pub sent: Option<Instant>,
    /// The frame as the last draw left it.
    pub shown: Layout,
    /// The pictures the transcript has turned into pixels. A memo of work
    /// already done, like [`Layout`], and holding no more of them than the
    /// terminal itself is asked to hold.
    pub decoded: Decoded,
    /// The pictures this session's own words named, read in once each (M51).
    /// A memo like [`Ui::decoded`], and the bound on how many a run of words
    /// may send this surface after ([`crate::graphics::linked`]).
    pub linked: Linked,
    /// A newer release than this build, as the start-up check found it (M63).
    pub welcome: crate::welcome::Details,
    /// The widest a line of prose is drawn, where a person set a measure
    /// (`tui.measure`, [`crate::settings`]; design §7). Terminal-side like the
    /// fold and the scroll, and read once at the start: it is a fact about
    /// this machine, not about the conversation.
    pub measure: Option<usize>,
    /// The paths the `@` dropdown ranks, walked when the first one asks.
    files: Files,
}

/// One reading of a directory, kept until the session's own directory changes.
#[derive(Debug, Default)]
struct Files {
    cwd: String,
    paths: Vec<String>,
}

impl Ui {
    pub fn sync_panes(&mut self, state: &bingo_sdk::SessionState) {
        self.placed.panes.sync(state, &mut self.controls.focus);
    }

    pub fn new(history: Vec<String>, _now: Instant) -> Self {
        Self {
            controls: controls::Controls::new(history),
            placed: Placed::default(),
            scroll: Scroll::default(),
            notices: Vec::new(),
            block: None,
            embers: crate::embers::Embers::default(),
            folds: Folds::new(),
            focused: true,
            switched: None,
            sent: None,
            shown: Layout::default(),
            decoded: Decoded::default(),
            linked: Linked::default(),
            welcome: crate::welcome::Details::default(),
            measure: None,
            files: Files::default(),
        }
    }

    pub fn notify(&mut self, level: Level, text: impl Into<String>, now: Instant) {
        self.say(level, text.into(), None, now);
    }

    /// A notice that names what it is about — the line an intent carried,
    /// said after it in dim.
    pub fn notify_about(&mut self, level: Level, text: String, about: String, now: Instant) {
        self.say(level, text, Some(about), now);
    }

    /// One notice is read at a time; the next takes the slot when this one has
    /// left it.
    fn say(&mut self, level: Level, text: String, about: Option<String>, now: Instant) {
        let free = self.notices.is_empty();
        self.notices.push(Notice {
            level,
            text,
            about,
            shown: free.then_some(now),
        });
    }

    /// The notice on the status line, while one is being said.
    pub fn notice(&self) -> Option<&Notice> {
        self.notices.first()
    }

    /// Take a notice back. What the opening had to say about the terminal can
    /// turn out to be wrong — a passthrough that answered after the probe gave
    /// up (M60 brick 2) — and a wrong word left standing is worse than none.
    /// Whichever is waiting behind it takes the slot on the next `expire`.
    pub fn withdraw(&mut self, text: &str) {
        self.notices.retain(|notice| notice.text != text);
    }

    /// Drop what has run out and start what was waiting for it: a notice past
    /// its window, a layer that has finished leaving. Drawing never mutates,
    /// so the loop calls this.
    pub fn expire(&mut self, now: Now) {
        if self.notices.first().is_some_and(|n| n.gone(now.instant)) {
            self.notices.remove(0);
        }
        if let Some(next) = self.notices.first_mut()
            && next.shown.is_none()
        {
            next.shown = Some(now.instant);
        }
        self.focus.expire(now);
    }

    /// How far the light along the box's border has come since a line was
    /// sent, or nothing at all once it has passed — and nothing where nothing
    /// may move (§6).
    pub fn sending(&self, now: Now) -> Option<f32> {
        let sent = self.sent.filter(|_| now.motion)?;
        match Anim::new(sent, IGNITION).progress(now.instant) {
            done if done >= 1.0 => None,
            come => Some(come),
        }
    }

    /// Whether the transcript is still crossfading into the session a person
    /// has just stepped into (§6).
    pub fn crossfading(&self, now: Now) -> bool {
        now.motion && self.switched.is_some_and(|at| now.since(at) < SWITCH)
    }

    pub fn next_change(&self, now: Now) -> Option<Instant> {
        let exit = self
            .armed
            .map(|at| at + EXIT_WINDOW)
            .filter(|at| *at > now.instant);
        let send = self
            .sent
            .filter(|_| self.sending(now).is_some())
            .map(|at| (now.instant + FRAME).min(at + IGNITION));
        let crossfade = self
            .switched
            .filter(|_| self.crossfading(now))
            .map(|at| at + SWITCH);
        let drag = self
            .select
            .dragging
            .filter(|_| self.focus.selection().is_some())
            .map(|held| held.stepped + select::EDGE_PACE);
        let intro = self
            .focus
            .intro()
            .filter(|intro| !intro.over(now))
            .map(|_| now.instant + FRAME);
        [exit, send, crossfade, drag, intro]
            .into_iter()
            .flatten()
            .min()
    }

    /// Every command the dropdown may offer: the surface's own and the
    /// kernel's, in that order.
    pub fn commands(&self) -> Vec<CommandSpec> {
        let mut all = commands::local_specs();
        all.extend(self.catalogs.commands.iter().cloned());
        all
    }

    /// The dropdown's rows for the line being typed: a `/` command, or an `@`
    /// mention — the names `mentions` says this session can reach, then the
    /// paths under its own directory. Empty means no dropdown.
    ///
    /// Who can be reached is the tree's to say ([`crate::mentions`]), so it is
    /// handed in rather than read here: what is the surface's own is the caret
    /// and the memo of the walk, and neither of those is the tree.
    pub fn suggestions(&self, cwd: &str, mentions: &[String]) -> Vec<Suggestion> {
        if self.menu.dismissed || self.listing() {
            return Vec::new();
        }
        let line = self.composer.text();
        match complete::mention(line) {
            Some(partial) => self.mentioned(cwd, partial, line, mentions),
            None => commands::suggestions(line, &self.commands(), &self.catalogs.values),
        }
    }

    /// Read directory names before entering the pure layout pass or answering a key.
    pub fn prepare(&mut self, cwd: &str) {
        if complete::mention(self.composer.text()).is_some() && self.files.cwd != cwd {
            self.files = Files {
                cwd: cwd.to_string(),
                paths: complete::walk(std::path::Path::new(cwd)),
            };
        }
    }

    fn mentioned(
        &self,
        cwd: &str,
        partial: &str,
        line: &str,
        mentions: &[String],
    ) -> Vec<Suggestion> {
        let paths = if self.files.cwd == cwd {
            self.files.paths.as_slice()
        } else {
            &[]
        };
        complete::offered(line, partial, mentions, paths)
    }

    pub fn selected_suggestion(&self, cwd: &str, mentions: &[String]) -> Option<Suggestion> {
        let rows = self.suggestions(cwd, mentions);
        rows.get(self.menu.selected.min(rows.len().saturating_sub(1)))
            .cloned()
    }

    /// Whether the input box is the list's query line rather than a message
    /// being written (M58). What is typed there names a session, so no `/` or
    /// `@` dropdown answers it and none is drawn over it.
    pub fn listing(&self) -> bool {
        self.focus.captures() && matches!(*self.focus.open(), Open::Switcher(_))
    }

    /// The composer changed: the dropdown reopens and the history walk ends.
    pub fn edited(&mut self) {
        self.menu = Menu::default();
        self.history.reset();
    }

    /// Whether a second ctrl+c would leave.
    pub fn exit_armed(&self, now: Instant) -> bool {
        self.armed
            .is_some_and(|at| now.duration_since(at) < EXIT_WINDOW)
    }

    /// How tall the transcript came out last frame and how many rows it had
    /// to show it in — what a scroll is measured against.
    pub fn transcript(&self) -> (usize, usize) {
        let painted = &self.shown;
        (painted.height, painted.regions.transcript.height as usize)
    }

    /// The whole transcript as the last frame rendered it, one string a line:
    /// what a search looks through.
    pub fn transcript_text(&self) -> Vec<String> {
        let painted = &self.shown;
        painted
            .blocks
            .window(0, painted.height)
            .iter()
            .map(ToString::to_string)
            .collect()
    }

    /// The rows a page key moves by: the screenful a person is looking at.
    pub fn page(&self) -> usize {
        self.transcript().1.max(1)
    }

    /// A drag held past an edge of the transcript: scroll towards it, and take
    /// the run's far end to the line now drawn at that edge.
    ///
    /// It remembers that the hand is still out there — unless what the edge
    /// reached is the transcript's own first or last line, where a hand may go
    /// on holding but there is nothing further to reach, and the frames
    /// another step would cost are not owed.
    ///
    /// The far end is measured against where the scroll is *going*, never
    /// against where its ease has reached: a drag sends events far faster than
    /// the ease is long, and the run would fall behind the view it is moving.
    pub fn drag_edge(&mut self, edge: Edge, column: usize, now: Now) {
        let (height, rows) = self.transcript();
        self.scroll.by(edge.lines(), height, rows, now.instant);
        let line = edge.line(self.scroll.target(height, rows), rows, height);
        self.extend_selection(Cell { line, column });
        self.select.dragging = edge.beyond(line, height).then_some(Dragging {
            edge,
            stepped: now.instant,
        });
    }

    /// One line more of a drag the hand is still holding past an edge, once
    /// [`select::EDGE_PACE`] has come round again. The loop asks every frame,
    /// and a frame is owed for as long as this has something to do.
    pub fn drag_step(&mut self, now: Now) {
        if self.focus.selection().is_none() {
            self.select.dragging = None;
            return;
        }
        let Some(held) = self.select.dragging else {
            return;
        };
        if now.since(held.stepped) < select::EDGE_PACE {
            return;
        }
        self.drag_edge(held.edge.step(), self.head_column(), now);
    }

    /// The column the run's far end is at: a drag walking past an edge keeps
    /// the column the pointer left the region at.
    fn head_column(&self) -> usize {
        self.focus.selection().map_or(0, |run| run.head.column)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::{later, scene};

    fn ui() -> Ui {
        Ui::new(Vec::new(), Instant::now())
    }

    #[test]
    fn a_notice_lives_its_window_and_the_frames_it_arrives_and_leaves_in() {
        let (mut ui, now) = scene();
        ui.notify(Level::Warn, "careful", now.instant);
        let life = (NOTICE_FADE + NOTICE + NOTICE_FADE).as_millis() as i64;
        ui.expire(later(now, life - 1));
        assert_eq!(ui.notices.len(), 1);
        ui.expire(later(now, life));
        assert!(ui.notices.is_empty());
    }

    #[test]
    fn a_notice_fades_in_over_two_frames_holds_and_fades_out() {
        let (mut ui, now) = scene();
        ui.notify(Level::Warn, "careful", now.instant);
        let at = |ms| {
            ui.notice()
                .and_then(|notice| notice.strength(later(now, ms)))
        };
        assert_eq!(at(0), Some(0.0), "it starts dim");
        assert_eq!(at(33), Some(0.5), "and arrives over two frames");
        assert_eq!(at(66), Some(1.0));
        assert_eq!(at(2_000), Some(1.0), "held while it is read");
        assert_eq!(at(4_066), Some(1.0), "to the end of its window");
        assert_eq!(at(4_099), Some(0.5), "then it leaves the same way");
        assert_eq!(at(4_132), Some(0.0));
    }

    #[test]
    fn a_still_surface_says_a_notice_at_full_strength() {
        let (mut ui, now) = scene();
        ui.notify(Level::Warn, "careful", now.instant);
        let still = crate::test_support::still(now);
        assert_eq!(ui.notice().and_then(|n| n.strength(still)), Some(1.0));
    }

    #[test]
    fn the_next_notice_waits_until_the_first_has_left() {
        let (mut ui, now) = scene();
        ui.notify(Level::Warn, "first", now.instant);
        ui.notify(Level::Info, "second", now.instant);
        assert_eq!(
            ui.notice().map(|n| n.text.clone()).as_deref(),
            Some("first")
        );
        assert_eq!(
            ui.notices[1].strength(now),
            None,
            "the one behind it is not being said at all"
        );

        let life = (NOTICE_FADE + NOTICE + NOTICE_FADE).as_millis() as i64;
        ui.expire(later(now, life));
        assert_eq!(
            ui.notice().map(|n| n.text.clone()).as_deref(),
            Some("second")
        );
        assert_eq!(
            ui.notices[0].strength(later(now, life)),
            Some(0.0),
            "and it arrives from the beginning of its own life"
        );
    }

    /// M60 brick 2: a word the opening said and the terminal then disproved is
    /// taken back, and whatever was waiting behind it takes the slot.
    #[test]
    fn a_notice_the_terminal_disproved_is_withdrawn() {
        let (mut ui, now) = scene();
        ui.notify(Level::Info, "the passthrough is off", now.instant);
        ui.notify(Level::Warn, "second", now.instant);
        ui.withdraw("the passthrough is off");
        assert_eq!(
            ui.notice().map(|n| n.text.clone()).as_deref(),
            Some("second")
        );
        ui.expire(now);
        assert_eq!(
            ui.notices[0].strength(now),
            Some(0.0),
            "and it arrives from the beginning of its own life"
        );
        ui.withdraw("nobody said that");
        assert_eq!(ui.notices.len(), 1, "and only what was said is taken back");
    }

    #[test]
    fn the_exit_arm_lapses() {
        let now = Instant::now();
        let mut ui = ui();
        ui.armed = Some(now);
        assert!(ui.exit_armed(now + Duration::from_millis(1999)));
        assert!(!ui.exit_armed(now + EXIT_WINDOW));
    }

    #[test]
    fn dismissing_the_dropdown_hides_it_until_the_next_edit() {
        let mut ui = ui();
        ui.composer.insert("/he");
        assert!(!ui.suggestions("/tmp/project", &[]).is_empty());
        ui.menu.dismissed = true;
        assert!(ui.suggestions("/tmp/project", &[]).is_empty());
        ui.edited();
        assert!(!ui.suggestions("/tmp/project", &[]).is_empty());
    }
}
