//! The activity band: the rows between the transcript and the input box
//! (design §3), which are what is going on rather than what was said — the
//! verb row while a turn runs, what the session is waiting on between turns
//! (M82), the task list under it or in its place (plugin activity slots, M74),
//! and whatever the person has queued behind it. What the last turn cost is
//! not here: it closes the transcript ([`crate::transcript::closing`]).
//!
//! Two rows are always held — the air and the verb row's slot — so a turn
//! starting or ending moves nothing; what the slot says is the one thing that
//! changes. Everything here is derived from the tree of states and the clock,
//! so a frame is a pure function of the two.

use bingo_sdk::{Origin, QueueEntry, SessionState};
use ratatui::text::{Line, Span};

use crate::clock::{self, Now};
use crate::tree::{self, Scope, Tree, Wants};
use crate::ui::Ui;
use crate::{theme, transcript, views};

/// How long a turn must have run before it is worth a row of its own (§6).
const ACTIVITY_AFTER: std::time::Duration = std::time::Duration::from_millis(300);
/// What the activity row's verb becomes once a person has asked the turn to
/// stop: bingo's own words are for what it chose to do, and this it did not.
pub const STOPPING: &str = "Stopping";
/// One breath of bingo's presence while it is thinking: the pace between the
/// other two, and the one a turn starts at (§6).
const BREATH: std::time::Duration = std::time::Duration::from_millis(1600);
/// The breath while words are arriving: quicker, because something is.
const BREATH_ARRIVING: std::time::Duration = std::time::Duration::from_millis(900);
/// The breath while a tool holds the turn: slower, because the waiting is
/// somebody else's and the row says so.
const BREATH_BLOCKED: std::time::Duration = std::time::Duration::from_millis(2200);
/// One visible phase of a terminal caret.
const BLINK: std::time::Duration = std::time::Duration::from_millis(530);
/// bingo's own words for working, selected from the turn id.
const VERBS: [&str; 10] = [
    "kindling",
    "stoking",
    "smouldering",
    "foraging",
    "tracking",
    "burrowing",
    "sniffing around",
    "pouncing",
    "sifting",
    "tinkering",
];

pub fn next_change(tree: &Tree, now: Now) -> Option<std::time::Instant> {
    tree.sessions().any(SessionState::busy).then(|| {
        if now.motion {
            now.instant + clock::FRAME
        } else {
            clock::next_tick(now, std::time::Duration::from_secs(1))
        }
    })
}

/// The rows between the transcript and the input box: what the turn is doing
/// — or, between turns, what it is waiting on — the session's task list, and
/// whatever the person has queued behind it.
pub(crate) fn components(
    tree: &Tree,
    ui: &Ui,
    width: usize,
    now: Now,
    pulls: &mut Vec<crate::pulled::Key>,
) -> views::Drawing {
    crate::layout::count::band(2);
    let state = tree.viewed();
    let slots = bingo_sdk::slot::placed(state);
    let mut out = band(tree, ui, &slots, width, now, pulls);
    out.lines.extend(queued(state));
    // A blank row between the transcript and these, as between any two blocks
    // (§3): they are not the tail of what was said, they are what is going on.
    if !out.lines.is_empty() {
        out.lines
            .insert(0, ui.embers.row(state, now, width, !ui.scroll.following()));
    }
    out
}

/// The verb row with the list hung under it while the turn shows one; the
/// summary with the list standing under it while there are tasks and no
/// turn runs; nothing at all otherwise (M74, Claude Code's own shape). One
/// row's slot either way (§3: nothing jumps). `ctrl+t` keeps the rows and
/// the summary off the band and leaves the verb: the task being done is
/// still what the turn is doing. What the last turn cost is not the band's:
/// it closes the transcript ([`crate::transcript::closing`]).
///
/// The turn's row comes first, the wait after it (M82) — the wake that ends
/// a wait opens a turn, and that turn is what the row is then for.
fn band(
    tree: &Tree,
    ui: &Ui,
    slots: &[bingo_sdk::slot::Placed],
    width: usize,
    now: Now,
    pulls: &mut Vec<crate::pulled::Key>,
) -> views::Drawing {
    let verb = slots.iter().find_map(|placed| match &placed.slot {
        bingo_sdk::slot::Slot::Activity { verb, .. } => verb.as_deref(),
        _ => None,
    });
    let mut next = None;
    let mut site = crate::component::Site {
        state: tree.viewed(),
        memo: &ui.placed.pulled,
        width,
        pulls,
        now,
        next: &mut next,
    };
    let mut out = if let Some(line) = crate::ask::drawing::activity(tree, ui) {
        vec![line]
    } else {
        working(tree.viewed(), ui, verb, now, &mut site)
    };
    if out.is_empty() {
        out.extend(waiting(tree, now));
    }
    let running = !out.is_empty();
    let mut out = views::Drawing { lines: out, next };
    if !ui.activity_hidden {
        for placed in slots {
            if let bingo_sdk::slot::Slot::Activity { view, .. } = &placed.slot {
                let Some(view) = ui.placed.pulled.view(view, tree.viewed(), width as u16) else {
                    out.lines.push(Line::from(theme::ellipsis()));
                    continue;
                };
                let view = match (running, view) {
                    (true, bingo_sdk::View::Panel { child, .. }) => child,
                    _ => view,
                };
                let lines = activity_view(view, width, now);
                out.append(lines.map_lines(|lines| {
                    if running {
                        hung(lines)
                    } else {
                        standing(lines)
                    }
                }));
            }
        }
    }
    out
}

fn activity_view(view: &bingo_sdk::View, width: usize, now: Now) -> views::Drawing {
    use bingo_sdk::View;
    match view {
        View::Panel { title, child } => {
            let mut out = vec![Line::from(Span::styled(title.clone(), theme::dim()))];
            activity_view(child, width, now).map_lines(|lines| {
                out.extend(lines);
                out
            })
        }
        View::Box { children, .. } => children
            .iter()
            .map(|v| activity_view(v, width, now))
            .collect(),
        View::Tree { nodes } if nodes.iter().all(|n| n.children.is_empty()) => nodes
            .iter()
            .map(|node| activity_node(node, width))
            .collect::<Vec<_>>()
            .into(),
        View::Text { children, style } => {
            let mut style = style.clone();
            if style.color.is_none() {
                style.color = Some("dim".into());
            }
            views::marked(
                &View::Text {
                    children: children.clone(),
                    style,
                },
                width,
                &views::Marks::at(now),
            )
        }
        _ => views::marked(view, width, &views::Marks::at(now)),
    }
}

fn activity_node(node: &bingo_sdk::TreeNode, width: usize) -> Line<'static> {
    use bingo_sdk::Tone;
    use unicode_width::UnicodeWidthStr;
    let (index, mark, words) = match node.tone {
        Tone::Neutral => (0, theme::text(), theme::text()),
        Tone::Attention => (1, theme::presence(), theme::text().patch(theme::bold())),
        Tone::Good => (2, theme::good(), theme::dim().patch(theme::struck())),
        Tone::Bad => (0, theme::bad(), theme::bad()),
    };
    let glyph = theme::tasks()[index];
    let mut spans = vec![
        Span::styled(format!("{glyph} "), mark),
        Span::styled(
            views::clip(&node.label, width.saturating_sub(glyph.width() + 1)),
            words,
        ),
    ];
    if let Some(badge) = &node.badge {
        spans.push(Span::styled(format!(" — {badge}"), theme::dim()));
    }
    Line::from(spans)
}

fn hung(rows: Vec<Line<'static>>) -> Vec<Line<'static>> {
    let connector = format!("  {}  ", theme::connector());
    let indent = " ".repeat(connector.chars().count());
    rows.into_iter()
        .enumerate()
        .map(|(at, row)| {
            let mut spans = vec![Span::styled(
                if at == 0 {
                    connector.clone()
                } else {
                    indent.clone()
                },
                theme::dim(),
            )];
            spans.extend(row.spans);
            Line::from(spans)
        })
        .collect()
}

fn standing(rows: Vec<Line<'static>>) -> Vec<Line<'static>> {
    rows.into_iter().map(|row| views::indent(row, 2)).collect()
}

/// The lines the person is holding behind the turn, dim, each under its `>`.
fn queued(state: &SessionState) -> Vec<Line<'static>> {
    state
        .queue
        .iter()
        .filter(|entry| pending(&entry.origin))
        .map(|entry| {
            Line::from(Span::styled(
                format!("{} {}{}", theme::user(), entry.preview, waits(entry)),
                theme::dim(),
            ))
        })
        .collect()
}

/// The tag a row wears when the running turn will not be steered with it: a
/// line that asked to wait for the turn to end, and a command, which has
/// always waited for one (ADR-0008 §2, amended M68). `steerable` is the one
/// reading of that, so there is nothing else here to keep in step with it.
fn waits(entry: &QueueEntry) -> &'static str {
    match entry.steerable {
        true => "",
        false => " (waits)",
    }
}

/// Whether a queued input is a message the person is waiting to send. A
/// subsystem's entry — a room's post, a spawn's brief, a job reporting in — is
/// a steer in flight rather than something pending (ADR-0028), so it is drawn
/// nowhere here; the turn that absorbs it shows it in the transcript as the
/// quiet notice it is. The boundary is the transcript's own set, so an unknown
/// surface fails to the loud, person's side in both places alike.
fn pending(origin: &Origin) -> bool {
    matches!(
        transcript::voice(origin),
        bingo_sdk::Voice::Said | bingo_sdk::Voice::Note
    )
}

/// `▍ Kindling… (4s · ↓ 1.2k)` — but only once the
/// turn has been at it for [`ACTIVITY_AFTER`]: a turn that answers at once
/// says nothing at all, because a row that flashes reports nothing (§6).
///
/// A turn a person has asked to stop reads `▍ Stopping… (4s · ↓ 1.2k)`
/// from the frame the key was pressed and dims its caret and keeps its clock
/// until `TurnCompleted` takes the row away. The keys a turn answers are the
/// KeyBar's to say, never this row's.
///
/// While a task on the list is in progress the verb is that task's own words
/// — `▍ Writing the plan…` — as Claude Code's spinner reads (M74); bingo's
/// own word is for a turn that has not said what it is doing.
fn working(
    state: &SessionState,
    ui: &Ui,
    placed_verb: Option<&str>,
    now: Now,
    site: &mut crate::component::Site<'_>,
) -> Vec<Line<'static>> {
    let Some(turn) = state.turn.as_ref() else {
        return Vec::new();
    };
    let elapsed = now.past(turn.started_at);
    let stopping = ui.stop_asked.as_ref() == Some(&turn.id);
    // A row that answers a key is never held back by the delay that spares a
    // fast turn its flash.
    if elapsed < ACTIVITY_AFTER && !stopping {
        return Vec::new();
    }
    let props = bingo_sdk::component::Props::Spinner(bingo_sdk::component::SpinnerProps {
        word: placed_verb.unwrap_or_else(|| verb(&turn.id)).into(),
        message: stopping.then(|| STOPPING.into()),
        suffix: String::new(),
        mode: if stopping { "stopping" } else { "working" }.into(),
    });
    let mut lines = site.draw("Spinner", &props, |props, _| {
        spinner(props, state, stopping, now)
    });
    let facts =
        bingo_sdk::component::Props::TurnProgress(bingo_sdk::component::TurnProgressProps {
            elapsed_ms: elapsed.as_secs().saturating_mul(1000),
            tokens_out: turn.usage.output_tokens,
            interruptible: !stopping,
            stopping,
        });
    let tail = site.draw(&format!("TurnProgress:{}", turn.id), &facts, progress);
    append_tail(&mut lines, tail);
    if let Some(retry) = turn.retrying
        && let Some(last) = lines.last_mut()
    {
        last.spans.push(Span::styled(
            format!(" retrying {}/{}", retry.attempt, retry.max),
            theme::presence(),
        ));
    }
    lines
}

/// `▍ Waiting for 2 background agents to finish` — what an idle session says
/// while agents it started are still at work (M82). A turn that ended to be
/// woken (M81) is not finished work, and the row that left with
/// `TurnCompleted` made it look finished: an empty composer over a dim
/// `2 running` in the status line, which the eye does not find.
///
/// The wait is not a turn, so it wears none of a turn's furniture: no `esc to
/// interrupt`, because there is no turn to end, and no clock and no token
/// count, which are what a turn has spent. The caret breathes at
/// [`BREATH_BLOCKED`] — the pace that already means the waiting is somebody
/// else's — and the input box's border stays dim, because nothing is arriving
/// here ([`crate::view`]'s `border` reads `busy`, which this session is not).
///
/// What it counts is what hangs *under* this session, not every other row the
/// status line counts: a child looking up at a running parent is waiting on
/// nothing of its own.
fn waiting(tree: &Tree, now: Now) -> Option<Line<'static>> {
    let viewed = tree.viewed();
    if viewed.busy() {
        return None;
    }
    let running = tree::count(tree, Wants::Running, Scope::Under(&viewed.summary.id))?;
    Some(Line::from(vec![
        Span::styled(
            format!(" {} ", theme::mark()),
            breathing_at(BREATH_BLOCKED, now),
        ),
        Span::styled(waiting_for(running), theme::text()),
    ]))
}

/// The row's words, in the number of agents there are.
fn waiting_for(running: usize) -> String {
    format!(
        "Waiting for {running} background agent{} to finish",
        if running == 1 { "" } else { "s" }
    )
}

/// How long one light takes to cross the working word and come round again.
/// Slower than the caret's breath, so the two are read as two things.
const BEAM: std::time::Duration = std::time::Duration::from_millis(2400);

/// bingo's word with one light walking across it while the turn runs (user,
/// 2026-09-05: the word wanted the beam the border and a landed call have).
/// The crest wears the glow and the rest of the word is `text`, so a frame
/// with no motion is the word alone.
fn beamed(word: String, now: Now) -> Vec<Span<'static>> {
    if !now.motion {
        return vec![Span::styled(word, theme::text())];
    }
    let t = clock::phase(now, BEAM);
    let width = word.chars().count();
    word.chars()
        .enumerate()
        .map(|(at, c)| {
            let lit = clock::sweep(t, at, width);
            Span::styled(c.to_string(), theme::comet(1.0 - lit))
        })
        .collect()
}

fn spinner(
    props: &bingo_sdk::component::Props,
    state: &SessionState,
    stopping: bool,
    now: Now,
) -> Vec<Line<'static>> {
    let bingo_sdk::component::Props::Spinner(props) = props else {
        return Vec::new();
    };
    let mut spans = vec![Span::styled(
        format!(" {} ", theme::mark()),
        caret_style(state, stopping, now),
    )];
    let suffix = if props.suffix == "…" {
        theme::ellipsis()
    } else {
        &props.suffix
    };
    spans.extend(beamed(
        format!(
            "{}{suffix}",
            props.message.as_deref().unwrap_or(&props.word)
        ),
        now,
    ));
    vec![Line::from(spans)]
}

fn progress(props: &bingo_sdk::component::Props, _: usize) -> Vec<Line<'static>> {
    let bingo_sdk::component::Props::TurnProgress(props) = props else {
        return Vec::new();
    };
    vec![Line::from(Span::styled(
        format!(" · {}s", props.elapsed_ms / 1000),
        theme::dim(),
    ))]
}

fn append_tail(lines: &mut Vec<Line<'static>>, tail: Vec<Line<'static>>) {
    let mut tail = tail.into_iter();
    if let Some(first) = tail.next() {
        if let Some(last) = lines.last_mut() {
            last.spans.extend(first.spans);
        } else {
            lines.push(first);
        }
    }
    lines.extend(tail);
}

/// bingo's own word for what it is doing (§4), drawn once per turn from the
/// turn's own id — so the same turn always reads the same way and a test can
/// name what it will say.
fn verb(turn: &bingo_sdk::TurnId) -> &'static str {
    VERBS[seed(turn.as_str()) % VERBS.len()]
}

/// FNV-1a: a stable spread over the words without a dependency to make one.
fn seed(id: &str) -> usize {
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for byte in id.as_bytes() {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    hash as usize
}

/// The mark reports the phase; the input border keeps its existing breath.
pub(crate) fn caret_style(state: &SessionState, stopping: bool, now: Now) -> ratatui::style::Style {
    if stopping || !state.busy() {
        return theme::dim();
    }
    if state.transcript.items().iter().any(|item| arriving(item)) {
        return theme::glow();
    }
    if state.transcript.items().iter().any(|item| blocking(item)) {
        return breathing_at(BREATH_BLOCKED, now);
    }
    let elapsed = state
        .turn
        .as_ref()
        .map(|turn| now.past(turn.started_at))
        .unwrap_or_default();
    if now.motion && (elapsed.as_millis() / BLINK.as_millis()) % 2 == 1 {
        theme::dim()
    } else {
        theme::presence()
    }
}

/// The input border keeps its breath while the caret names the turn phase.
pub(crate) fn breathing(state: &SessionState, now: Now) -> ratatui::style::Style {
    breathing_at(breath_of(state), now)
}

/// The same breath at a pace the caller names — what the wait row takes,
/// having no turn to read one from. Still, it rests at `presence`: what
/// breathes is the brightness, not the fact that something is at work.
fn breathing_at(period: std::time::Duration, now: Now) -> ratatui::style::Style {
    match now.motion {
        true => theme::breath(clock::breath(now, period)),
        false => theme::presence(),
    }
}

/// How fast it breathes: the rhythm is what the turn is *doing*, so a pulse
/// says more than "a turn is running", which the row's presence already says
/// (§6). Words arriving are quick, a tool holding the turn is slow, and
/// thinking is the pace between them.
///
/// The phase is the wall clock's own turn of the period ([`clock::breath`]),
/// so a change of period changes where in the breath this frame lands. That
/// step is the state change itself, which is the one moment §6 allows a cue
/// to move — and it happens at most twice in a turn.
pub(crate) fn breath_of(state: &SessionState) -> std::time::Duration {
    if state.transcript.items().iter().any(|item| arriving(item)) {
        return BREATH_ARRIVING;
    }
    if state.transcript.items().iter().any(|item| blocking(item)) {
        return BREATH_BLOCKED;
    }
    BREATH
}

/// Whether an item is an answer still being said.
fn arriving(item: &bingo_sdk::Item) -> bool {
    matches!(
        item.body,
        bingo_sdk::ItemBody::Assistant { .. } | bingo_sdk::ItemBody::Reasoning { .. }
    ) && item.status == bingo_sdk::ItemStatus::Running
}

/// Whether an item is a call the turn is waiting on.
fn blocking(item: &bingo_sdk::Item) -> bool {
    matches!(item.body, bingo_sdk::ItemBody::ToolCall { .. })
        && item.status == bingo_sdk::ItemStatus::Running
}

#[cfg(test)]
pub(crate) fn lines(tree: &Tree, ui: &Ui, width: usize, now: Now) -> Vec<Line<'static>> {
    components(tree, ui, width, now, &mut Vec::new()).lines
}

#[cfg(test)]
mod tests {
    #[test]
    fn tokens_do_not_compete_with_the_activity_clock() {
        let state = folded(vec![
            frame(1, started("trn_1")),
            frame(
                2,
                bingo_sdk::Event::TurnUsage {
                    turn: bingo_sdk::TurnId::from_raw("trn_1"),
                    usage: bingo_sdk::Usage {
                        output_tokens: 12,
                        ..Default::default()
                    },
                    context: Default::default(),
                },
            ),
        ]);
        let (mut ui, now) = scene();
        let screen = draw_tree(80, 24, &solo(&state), &mut ui, later(now, 4_000));
        assert!(!screen.contains("↓ 12"), "{screen}");
        assert!(screen.contains("4s"), "{screen}");
        assert!(!screen.contains("0.0k"), "{screen}");
    }

    use super::*;
    use crate::test_support::*;

    /// The band's one row, as the terminal drew it, without the quotes a test
    /// backend prints around each row or the padding it is drawn into.
    fn row(tree: &Tree, now: Now) -> String {
        let (mut ui, _) = scene();
        draw_tree(80, 24, tree, &mut ui, now)
            .lines()
            .find(|line| line.contains("Waiting for"))
            .map(|line| line.trim_matches('"').trim_end().to_string())
            .unwrap_or_default()
    }

    /// What an idle session says while the agents it started are still at
    /// work, counted the way a person counts (M82).
    #[test]
    fn a_session_idle_over_running_agents_says_how_many_it_waits_for() {
        let (_, now) = scene();
        let one = row(&waited_on(1), now);
        assert!(
            one.ends_with("Waiting for 1 background agent to finish"),
            "{one}"
        );
        let two = row(&waited_on(2), now);
        assert!(
            two.ends_with("Waiting for 2 background agents to finish"),
            "{two}"
        );
    }

    /// The wait is not a turn, so it wears none of a turn's furniture — every
    /// piece of which rides in the row's one pair of parentheses.
    #[test]
    fn the_wait_carries_no_key_no_clock_and_no_token_count() {
        let (_, now) = scene();
        let drawn = row(&waited_on(1), now);
        assert!(!drawn.contains('('), "{drawn}");
    }

    /// Nothing at work, nothing to wait for: an agent that has finished is
    /// not waited on, and a session with none never was.
    #[test]
    fn a_session_with_no_agent_at_work_says_nothing_at_all() {
        let (_, now) = scene();
        assert_eq!(row(&waited_on(0), now), "");
        let mut done = waited_on(1);
        done.fold.apply(&agent_frame(
            2,
            3,
            completed("trn_9", bingo_sdk::TurnStatus::Completed),
        ));
        assert_eq!(row(&done, now), "");
    }

    /// The row counts what hangs under this session, not everything else
    /// that is running: a child looking up at a running parent is waiting on
    /// nothing of its own. The crew strip likewise names only descendants.
    #[test]
    fn a_child_under_a_running_root_is_not_waiting_on_an_agent() {
        let (mut ui, now) = scene();
        let mut tree = folded_tree(vec![
            frame(1, started("trn_1")),
            child_frame(1, announced("reviewer")),
        ]);
        tree.show(&child_id());
        assert_eq!(row(&tree, now), "");
        let drawn = draw_tree(80, 24, &tree, &mut ui, now);
        assert!(!drawn.contains("running"), "{drawn}");
    }

    /// The wake that ends the wait opens a turn, and the turn's own row is
    /// what the band then says: one row's slot either way (§3).
    #[test]
    fn a_turn_on_the_viewed_session_takes_the_row_back_from_the_wait() {
        let (mut ui, now) = scene();
        let mut woken = waited_on(1);
        woken.fold.apply(&frame(2, started("trn_1")));
        let at = later(now, 1_600);
        assert_eq!(row(&woken, at), "");
        let drawn = draw_tree(80, 24, &woken, &mut ui, at);
        assert!(!drawn.contains("esc to interrupt"), "{drawn}");
    }

    /// The working word carries one light across itself while the turn runs:
    /// two frames apart, different cells are lit; with motion off it is the
    /// word in `text` and nothing else.
    #[test]
    fn the_working_word_is_beamed_while_the_turn_runs_and_plain_when_still() {
        let (_, now) = crate::test_support::scene();
        let moving = beamed("Rummaging".to_string(), now);
        assert_eq!(moving.len(), 9, "one span per character");
        let later = beamed(
            "Rummaging".to_string(),
            crate::test_support::later(now, 600),
        );
        assert_ne!(
            moving.iter().map(|s| s.style).collect::<Vec<_>>(),
            later.iter().map(|s| s.style).collect::<Vec<_>>(),
            "the light has moved"
        );
        let still = beamed("Rummaging".to_string(), crate::test_support::still(now));
        assert_eq!(still.len(), 1);
        assert_eq!(still[0].style, theme::text());
    }
}
