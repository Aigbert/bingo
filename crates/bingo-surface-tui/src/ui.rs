//! Everything that is the surface's own and nothing that is the session's.
//! The transcript, the turn, the queue and the open interactions all live in
//! `SessionState`; what lives here is what a second client would not share —
//! where the caret is, what is scrolled, what was armed and when.
//!
//! Every time-dependent decision takes `now`, so a test never sleeps.

use std::time::{Duration, Instant};

use bingo_sdk::{CommandSpec, Level, Tone, View, view::TextChild};

use crate::clock::{FRAME, Now};
use crate::commands::{self, Suggestion};
use crate::complete;
use crate::fold::Folds;
use crate::graphics::{Decoded, Linked};
use crate::layers::{self, Reveal};
use crate::layout::Layout;
use crate::rewind::Rewind;
use crate::scroll::Scroll;
use crate::select::{self, Cell, Dragging, Edge};

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

/// A toast has one wall-clock window, whether the surface draws it or not.
pub const NOTICE: Duration = TOAST_HOLD.saturating_add(TOAST_FADE);
const TOAST_HOLD: Duration = Duration::from_millis(2400);
/// A second ctrl+c within this window leaves.
pub use crate::keys::EXIT_WINDOW;
/// A transcript that has just been stepped into crossfades through dim over
/// this long: two frames (§6).
pub const SWITCH: Duration = Duration::from_millis(2 * FRAME.as_millis() as u64);

#[derive(Clone, Debug)]
pub struct Message {
    pub about: Option<String>,
    pub parts: Vec<TextChild>,
    pub tone: Tone,
}

// Test assertions may read prose; the drawing always consumes the original parts.
#[cfg(test)]
pub struct Plain {
    pub text: String,
    pub tone: Tone,
}

fn tone(level: Level) -> Tone {
    match level {
        Level::Info => Tone::Neutral,
        Level::Warn => Tone::Attention,
        Level::Error => Tone::Bad,
    }
}

/// A message or a mode-key acknowledgement; the latter owns no session words.
#[derive(Clone, Debug)]
pub enum NoticeKind {
    Message(Message),
    Mode,
}
impl NoticeKind {
    pub fn hold(&self) -> Duration {
        match self {
            Self::Message(_) => TOAST_HOLD,
            Self::Mode => Duration::from_millis(2800),
        }
    }
}
pub const TOAST_FADE: Duration = Duration::from_millis(300);

#[derive(Clone, Debug)]
pub struct Notice {
    pub kind: NoticeKind,
    said: Instant,
    hold: Duration,
}
impl Notice {
    pub fn new(kind: NoticeKind, said: Instant) -> Self {
        let hold = kind.hold();
        Self { kind, said, hold }
    }
    #[cfg(test)]
    pub fn plain(&self) -> Option<Plain> {
        match &self.kind {
            NoticeKind::Message(message) => Some(Plain {
                text: message
                    .parts
                    .iter()
                    .map(TextChild::fold)
                    .collect::<Vec<_>>()
                    .join(" · "),
                tone: message.tone,
            }),
            NoticeKind::Mode => None,
        }
    }
    pub(crate) fn hold(&self) -> Duration {
        self.hold
    }
    pub(crate) fn said(&self) -> Instant {
        self.said
    }
    pub(crate) fn gone(&self, now: Instant) -> bool {
        now.saturating_duration_since(self.said) >= self.hold + TOAST_FADE
    }
    #[cfg(test)]
    pub fn strength(&self, now: Now) -> f32 {
        if self.gone(now.instant) { 0.0 } else { 1.0 }
    }
}

/// The command dropdown's own state; its rows are derived from the composer.
#[derive(Clone, Copy, Debug, Default)]
pub struct Menu {
    pub selected: usize,
    /// Esc closed it; typing opens it again.
    pub dismissed: bool,
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
    pub current: Option<crate::palette::Current>,
    pub plugins: Vec<bingo_sdk::CatalogEntry>,
    pub commands: Vec<bingo_sdk::CatalogEntry>,
    pub values: std::collections::BTreeMap<String, Vec<bingo_sdk::CatalogEntry>>,
}

impl Catalogs {
    pub(crate) fn specs(&self) -> Vec<CommandSpec> {
        commands::specs(&self.commands)
    }

    pub(crate) fn values(
        &self,
    ) -> std::collections::BTreeMap<String, Vec<bingo_sdk::CatalogEntry>> {
        let mut values = self.values.clone();
        values.insert("plugins".into(), self.plugins.clone());
        values
    }
}

/// What is over the frame. One at a time: focus moves into a layer and back
/// out, never sideways (§7).
#[derive(Clone, Debug, Default, PartialEq)]
pub enum Open {
    #[default]
    Nothing,
    Keys(Option<String>),
    Mode(Option<String>),
    Result {
        head: String,
        view: View,
        at: usize,
    },
    Palette(crate::palette::Opened),
    Sessions(crate::sessions::Opened),
    Browse(crate::browse::Opened),
    Reader(crate::browse::reader::Reader),
    Plugins(crate::plugins::Opened),
    /// The turns of this transcript, as a card above the input box.
    Rewind(Rewind),
}

impl Open {
    /// Whether the keyboard belongs to it while it is open. The lists and the
    /// panel answer keys; help is read while a person goes on typing.
    pub fn captures(&self) -> bool {
        matches!(
            self,
            Open::Keys(_)
                | Open::Mode(_)
                | Open::Result { .. }
                | Open::Sessions(_)
                | Open::Plugins(_)
                | Open::Browse(_)
                | Open::Reader(_)
                | Open::Palette(_)
                | Open::Rewind(_)
        )
    }

    /// How many frames its arrival takes: a card comes down, a sheet rises.
    fn frames(&self) -> u16 {
        match self {
            Open::Plugins(_) | Open::Palette(_) | Open::Rewind(_) => layers::CARD_FRAMES,
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
    pub browse: crate::browse::cache::Cache,
    pub panes: crate::pane::State,
    pub pulled: crate::pulled::Memo,
    pub toasts: crate::toast::State,
}

#[derive(Debug)]
pub struct Ui {
    pub wells: std::collections::BTreeMap<bingo_sdk::TurnId, bingo_sdk::ItemId>,
    pub placed: Placed,
    pub(crate) controls: controls::Controls,
    pub scroll: Scroll,
    /// The one notice being said: the latest wins the slot, and nothing waits
    /// behind it (ADR-0095 §2).
    pub notice: Option<Notice>,
    pub keyed_at: Instant,
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
    /// The region cap, inside which prose keeps its 100-cell measure
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

    pub fn new(history: Vec<String>, now: Instant) -> Self {
        Self {
            wells: Default::default(),
            controls: controls::Controls::new(history),
            placed: Placed::default(),
            scroll: Scroll::default(),
            notice: None,
            keyed_at: now,
            embers: crate::embers::Embers::default(),
            folds: Folds::new(),
            focused: true,
            switched: None,
            shown: Layout::default(),
            decoded: Decoded::default(),
            linked: Linked::default(),
            welcome: crate::welcome::Details::default(),
            measure: None,
            files: Files::default(),
        }
    }

    pub fn notify(&mut self, level: Level, text: impl Into<String>, now: Instant) {
        self.notify_parts(level, vec![TextChild::String(text.into())], now);
    }

    pub fn notify_parts(&mut self, level: Level, parts: Vec<TextChild>, now: Instant) {
        self.notify_for(level, parts, TOAST_HOLD, now);
    }

    pub fn notify_for(
        &mut self,
        level: Level,
        parts: Vec<TextChild>,
        hold: Duration,
        now: Instant,
    ) {
        self.say(
            Message {
                parts,
                tone: tone(level),
                about: None,
            },
            hold,
            now,
        );
    }

    /// What a command asked to say, in the inks its owner gave each part.
    pub fn tell(&mut self, said: bingo_sdk::toast::Say, now: Instant) {
        self.say(
            Message {
                parts: said.parts,
                tone: said.tone,
                about: None,
            },
            TOAST_HOLD,
            now,
        );
    }

    /// A notice that names what it is about — the line an intent carried,
    /// said after it in dim.
    pub fn notify_about(&mut self, level: Level, text: String, about: String, now: Instant) {
        self.say(
            Message {
                parts: vec![TextChild::String(text)],
                tone: tone(level),
                about: Some(about),
            },
            TOAST_HOLD,
            now,
        );
    }

    /// A newer notice replaces the old one; nothing waits to be said later.
    fn say(&mut self, message: Message, hold: Duration, now: Instant) {
        self.placed.toasts.discard();
        self.notice = Some(Notice {
            kind: NoticeKind::Message(message),
            said: now,
            hold,
        });
    }

    pub fn notify_mode(&mut self, now: Instant) {
        self.placed.toasts.discard();
        self.notice = Some(Notice::new(NoticeKind::Mode, now));
    }

    pub fn sync_toasts(&mut self, state: &bingo_sdk::SessionState, now: Now) -> bool {
        let alert = self.placed.toasts.sync(state, now);
        if let Some(notice) = &self.notice
            && self.placed.toasts.supersedes(notice.said())
        {
            self.notice = None;
        }
        alert
    }

    pub fn clear_notice(&mut self) {
        self.notice = None;
        self.placed.toasts.discard();
    }

    /// The local toast, while one is being said.
    pub fn notice(&self) -> Option<&Notice> {
        self.notice.as_ref()
    }

    /// Take a notice back. What the opening had to say about the terminal can
    /// turn out to be wrong — a passthrough that answered after the probe gave
    /// up (M60 brick 2) — and a wrong word left standing is worse than none.
    pub fn withdraw(&mut self, text: &str) {
        if self
            .notice()
            .is_some_and(|notice| matches!(&notice.kind, NoticeKind::Message(message) if message.parts == [TextChild::from(text)]))
        {
            self.notice = None;
        }
    }

    /// Drop what has run out: a notice past its window, a layer that has
    /// finished leaving. Drawing never mutates, so the loop calls this.
    pub fn expire(&mut self, now: Now) {
        if self.notice().is_some_and(|n| n.gone(now.instant)) {
            self.notice = None;
        }
        self.focus.expire(now);
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
        let crossfade = self
            .switched
            .filter(|_| self.crossfading(now))
            .map(|at| at + SWITCH);
        let drag = self
            .select
            .dragging
            .filter(|_| self.focus.selection().is_some())
            .map(|held| held.stepped + select::EDGE_PACE);
        [exit, crossfade, drag].into_iter().flatten().min()
    }

    /// Every command the dropdown may offer: the surface's own and the
    /// kernel's, in that order.
    pub fn commands(&self) -> Vec<CommandSpec> {
        let mut all = commands::local_specs();
        all.extend(self.catalogs.specs());
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
        if self.menu.dismissed || crate::palette::active(self) || crate::sessions::active(self) {
            return Vec::new();
        }
        let line = self.composer.text();
        match complete::mention(line) {
            Some(partial) => self.mentioned(cwd, partial, line, mentions),
            None => {
                let mut offered = commands::local_specs();
                offered.extend(
                    self.catalogs
                        .commands
                        .iter()
                        .filter(|entry| entry.meta["foldedInto"].as_str().is_none())
                        .filter_map(|entry| serde_json::from_value(entry.meta.clone()).ok()),
                );
                commands::suggestions(line, &offered)
            }
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

    /// The composer changed: the dropdown reopens and the history walk ends.
    pub fn edited(&mut self) {
        self.menu = Menu::default();
        self.history.reset();
    }

    /// Whether a second ctrl+c would leave.
    pub fn exit_armed(&self, now: Instant) -> bool {
        self.armed
            .is_some_and(|at| now.saturating_duration_since(at) < EXIT_WINDOW)
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

impl Ui {
    pub fn set_well(&mut self, turn: bingo_sdk::TurnId, item: Option<bingo_sdk::ItemId>) {
        match item {
            Some(item) => {
                self.wells.insert(turn, item);
            }
            None => {
                self.wells.remove(&turn);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::{later, scene};

    fn ui() -> Ui {
        Ui::new(Vec::new(), Instant::now())
    }

    fn said(ui: &Ui) -> Option<&str> {
        ui.notice().and_then(|notice| match &notice.kind {
            NoticeKind::Message(message) => match message.parts.first()? {
                TextChild::String(text) => Some(text.as_str()),
                TextChild::Node(_) => None,
            },
            NoticeKind::Mode => None,
        })
    }

    fn life() -> i64 {
        NOTICE.as_millis() as i64
    }

    #[test]
    fn a_notice_keeps_parts_tone_and_hold_from_birth() {
        let (mut ui, now) = scene();
        let hold = Duration::from_millis(1800);
        ui.notify_for(
            Level::Warn,
            vec!["cannot read".into(), "switch model".into()],
            hold,
            now.instant,
        );
        let notice = ui.notice().unwrap();
        let NoticeKind::Message(message) = &notice.kind else {
            panic!("message")
        };
        assert_eq!(
            message.parts,
            [TextChild::from("cannot read"), "switch model".into()]
        );
        assert_eq!(message.tone, Tone::Attention);
        assert_eq!(notice.hold(), hold);
        assert!(!notice.gone(now.instant + hold));
        assert!(notice.gone(now.instant + hold + TOAST_FADE));
    }

    #[test]
    fn a_notice_lives_its_window_and_the_frames_it_arrives_and_leaves_in() {
        let (mut ui, now) = scene();
        ui.notify(Level::Warn, "careful", now.instant);
        ui.expire(later(now, life() - 1));
        assert!(ui.notice.is_some());
        ui.expire(later(now, life()));
        assert!(ui.notice.is_none());
    }

    #[test]
    fn a_toast_expires_after_2700_milliseconds() {
        let (mut ui, now) = scene();
        ui.notify(Level::Warn, "careful", now.instant);
        for ms in [0, 33, 66, 2000, 2699] {
            assert_eq!(ui.notice().map(|n| n.strength(later(now, ms))), Some(1.0));
        }
        assert_eq!(ui.notice().map(|n| n.strength(later(now, 2700))), Some(0.0));
    }

    #[test]
    fn a_still_surface_says_a_notice_at_full_strength() {
        let (mut ui, now) = scene();
        ui.notify(Level::Warn, "careful", now.instant);
        let still = crate::test_support::still(now);
        assert_eq!(ui.notice().map(|n| n.strength(still)), Some(1.0));
    }

    /// M182 bug 6: five quick mode switches say five notices; only the mode
    /// it settled on is said, and nothing is replayed after it.
    #[test]
    fn rapid_notices_leave_only_the_last_one_said() {
        let (mut ui, now) = scene();
        let modes = [
            "acceptEdits",
            "plan",
            "bypassPermissions",
            "dontAsk",
            "default",
        ];
        for (step, mode) in modes.iter().enumerate() {
            ui.notify(Level::Info, *mode, later(now, step as i64 * 100).instant);
        }
        let last = later(now, 400);
        ui.expire(last);
        assert_eq!(said(&ui), Some("default"));
        ui.expire(later(last, life()));
        assert_eq!(said(&ui), None, "and nothing waits behind it");
    }

    #[test]
    fn a_notice_whose_moment_passed_behind_another_is_never_said() {
        let (mut ui, now) = scene();
        ui.notify(Level::Warn, "first", now.instant);
        ui.notify(Level::Info, "second", later(now, 1_000).instant);
        assert_eq!(
            said(&ui),
            Some("second"),
            "the newer word replaces the first"
        );
        for ms in (1_000..=life() + 2_000).step_by(250) {
            ui.expire(later(now, ms));
            assert_ne!(said(&ui), Some("first"), "at {ms} ms");
        }
        assert_eq!(said(&ui), None);
    }

    #[test]
    fn a_notice_expires_by_the_wall_clock_from_when_it_was_said() {
        let (mut ui, now) = scene();
        ui.notify(Level::Info, "said early", now.instant);
        // The first frame to look at it comes late: its life is not restarted.
        let looked = later(now, life() - 10);
        ui.expire(looked);
        assert_eq!(said(&ui), Some("said early"));
        let strength = ui.notice().map_or(1.0, |n| n.strength(looked));
        assert_eq!(strength, 1.0, "still inside its original window");
        ui.expire(later(now, life()));
        assert_eq!(
            said(&ui),
            None,
            "gone at its own deadline, not one drawn later"
        );
    }

    /// M60 brick 2: a word the opening said and the terminal then disproved is
    /// taken back.
    #[test]
    fn a_notice_the_terminal_disproved_is_withdrawn() {
        let (mut ui, now) = scene();
        ui.notify(Level::Info, "the passthrough is off", now.instant);
        ui.withdraw("nobody said that");
        assert_eq!(
            said(&ui),
            Some("the passthrough is off"),
            "only what was said is taken back"
        );
        ui.withdraw("the passthrough is off");
        assert_eq!(said(&ui), None);
    }

    #[test]
    fn the_exit_arm_lapses() {
        let now = Instant::now();
        let mut ui = ui();
        ui.armed = Some(now);
        assert!(ui.exit_armed(now + Duration::from_millis(1499)));
        assert!(!ui.exit_armed(now + EXIT_WINDOW));
    }

    #[test]
    fn dismissing_the_dropdown_hides_it_until_the_next_edit() {
        let mut ui = ui();
        ui.composer.insert("/cle");
        assert!(!ui.suggestions("/tmp/project", &[]).is_empty());
        ui.menu.dismissed = true;
        assert!(ui.suggestions("/tmp/project", &[]).is_empty());
        ui.edited();
        assert!(!ui.suggestions("/tmp/project", &[]).is_empty());
    }
}
