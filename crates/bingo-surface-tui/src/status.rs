//! The persistent mode, crew, context and model footer, plus transitional hints.
//! All facts come from the folded session and plugin slots.

use bingo_sdk::{Effort, SessionState};
use ratatui::text::{Line, Span};
use unicode_width::UnicodeWidthStr;

use crate::clock::{self, Now};
use crate::tree::{self, Scope, Tree, Wants};
use crate::ui::Ui;
use crate::{keys, theme};

/// Cells between two slots.
const GAP: usize = 2;
/// What the middle says when nothing else is true and nothing is typed.
pub const HINT: &str = keys::FOOTER_HINT;

/// The status line at this width.
pub fn components(
    tree: &Tree,
    ui: &Ui,
    width: usize,
    now: Now,
    pulls: &mut Vec<crate::pulled::Key>,
) -> (Line<'static>, Option<std::ops::Range<usize>>) {
    crate::layout::count::band(3);
    let mut site = crate::component::Site {
        state: tree.viewed(),
        memo: &ui.placed.pulled,
        width,
        pulls,
    };
    let right = right(tree, &mut site, now);
    let left = mode(tree.viewed(), &mut site);
    let mut crew = crate::crew::spans(tree, width, width, now);
    if crew.is_empty() {
        crew = vec![
            Span::styled(theme::glyphs().chrome.steps[1], theme::crew(0)),
            dimmed(" solo".into()),
        ];
    }
    let context = context_bar(tree.viewed().context);
    footer(left, crew, context, right, width)
}

fn footer(
    mut mode: Vec<Span<'static>>,
    mut crew: Vec<Span<'static>>,
    mut context: Vec<Span<'static>>,
    model: Vec<Span<'static>>,
    width: usize,
) -> (Line<'static>, Option<std::ops::Range<usize>>) {
    let room = width.saturating_sub(3);
    let fits = |crew: &[Span<'static>], context: &[Span<'static>]| {
        cells(&mode)
            + cells(crew)
            + cells(context)
            + cells(&model)
            + 3 * (usize::from(!crew.is_empty()) + usize::from(!context.is_empty()))
            + GAP
            <= room
    };
    if !fits(&crew, &context) {
        crew.clear();
    }
    if !fits(&crew, &context) {
        context.clear();
    }
    let crew_start = 1 + cells(&mode) + 3;
    let hit = (!crew.is_empty()).then_some(crew_start..crew_start + cells(&crew));
    for part in [crew, context] {
        if !part.is_empty() {
            mode.push(pad(3));
            mode.extend(part);
        }
    }
    let mut spans = vec![pad(1)];
    spans.extend(place(mode, Vec::new(), model, room).spans);
    spans.push(pad(2));
    (Line::from(clip(spans, width)), hit)
}

fn context_bar(context: Option<bingo_sdk::ContextUsage>) -> Vec<Span<'static>> {
    let Some(context) = context else {
        return vec![
            Span::styled(theme::glyphs().chrome.context[1].repeat(8), theme::ash()),
            dimmed(" ? / ?".into()),
        ];
    };
    let share = context.used as f64 / context.window.max(1) as f64;
    let fill = if context.window == 0 {
        0
    } else {
        (share * 8.0).round().clamp(1.0, 8.0) as usize
    };
    let [full, empty] = theme::glyphs().chrome.context;
    vec![
        Span::styled(
            full.repeat(fill),
            if share > 0.9 {
                theme::bad()
            } else {
                theme::presence()
            },
        ),
        Span::styled(empty.repeat(8 - fill), theme::ash()),
        dimmed(format!(
            " {} / {}",
            crate::count::count(context.used),
            crate::count::count(context.window)
        )),
    ]
}

/// Transitional hints keep their existing words until KeyBar owns this row.
pub fn hints(
    tree: &Tree,
    ui: &Ui,
    width: usize,
    now: Now,
    pulls: &mut Vec<crate::pulled::Key>,
) -> Line<'static> {
    let mut site = crate::component::Site {
        state: tree.viewed(),
        memo: &ui.placed.pulled,
        width,
        pulls,
    };
    let (spans, _) = middle(tree, ui, width, width, now, &mut site);
    Line::from(clip(spans, width))
}

fn mode(state: &SessionState, site: &mut crate::component::Site<'_>) -> Vec<Span<'static>> {
    let modes: Vec<String> = bingo_sdk::slot::placed(state)
        .into_iter()
        .filter_map(|placed| match placed.slot {
            bingo_sdk::slot::Slot::Mode { current, .. } => Some(current),
            _ => None,
        })
        .collect();
    let modes = if modes.is_empty() {
        vec!["default".into()]
    } else {
        modes
    };
    let props =
        bingo_sdk::component::Props::SessionMode(bingo_sdk::component::SessionModeProps { modes });
    site.draw("SessionMode", &props, |changed, _| {
        if changed == &props {
            return vec![Line::from(left(state))];
        }
        let bingo_sdk::component::Props::SessionMode(props) = changed else {
            return Vec::new();
        };
        vec![Line::from(Span::styled(
            props.modes.join(" · "),
            theme::mode(),
        ))]
    })
    .into_iter()
    .next()
    .unwrap_or_default()
    .spans
}

/// Every mode has a visible label, including the resting default.
fn left(state: &SessionState) -> Vec<Span<'static>> {
    let label = bingo_sdk::slot::placed(state)
        .into_iter()
        .find_map(|placed| match placed.slot {
            bingo_sdk::slot::Slot::Mode { current, .. } => Some(current),
            _ => None,
        })
        .unwrap_or_else(|| "default".into());
    let label = match label.as_str() {
        "acceptEdits" => "accept edits",
        "bypassPermissions" => "bypass",
        "dontAsk" => "don't ask",
        other => other,
    };
    let style = match label {
        "accept edits" => theme::good(),
        "plan" => theme::crew(2),
        "bypass" => theme::bad(),
        _ => theme::mode(),
    };
    let glyph = if label == "plan" {
        theme::glyphs().chrome.pause
    } else {
        theme::glyphs().mode
    };
    vec![Span::styled(format!("{glyph} {label}"), style)]
}

/// Only what is true now, in the order the eye should take it.
fn middle(
    tree: &Tree,
    ui: &Ui,
    _width: usize,
    _room: usize,
    now: Now,
    site: &mut crate::component::Site<'_>,
) -> (Vec<Span<'static>>, bool) {
    if ui.exit_armed(now.instant) {
        return (
            vec![Span::styled(crate::input::ARM_HINT, theme::text())],
            false,
        );
    }
    let notice = notice(ui, now);
    if !notice.is_empty() {
        return (join(notice), false);
    }
    let placed = placed_status(tree.viewed(), now);
    if !placed.is_empty() {
        return (join(placed), false);
    }
    if let Some(waiting) = repository_waiting(ui, now) {
        return (vec![waiting], false);
    }
    if let Some(waiting) = tree::count(tree, Wants::Attention, Scope::Others) {
        return (
            vec![Span::styled(
                format!("{waiting} needs you (ctrl+g)"),
                theme::attention(now),
            )],
            false,
        );
    }
    if ui.composer.is_empty() {
        let props =
            bingo_sdk::component::Props::PromptHint(bingo_sdk::component::PromptHintProps {
                is_draft: false,
                is_working: tree.viewed().busy(),
                hint: HINT.into(),
            });
        let lines = site.draw("PromptHint", &props, |props, _| {
            let bingo_sdk::component::Props::PromptHint(props) = props else {
                return Vec::new();
            };
            vec![Line::from(Span::styled(props.hint.clone(), theme::dim()))]
        });
        return (lines.into_iter().next().unwrap_or_default().spans, false);
    }
    (Vec::new(), false)
}

fn repository_waiting(ui: &Ui, now: Now) -> Option<Span<'static>> {
    let count = ui
        .catalogs
        .plugins
        .iter()
        .filter(|entry| entry.meta["state"]["kind"] == "waiting")
        .count();
    if count == 0 {
        return None;
    }
    let verb = if count == 1 { "waits" } else { "wait" };
    Some(Span::styled(
        format!("{count} from this repo {verb} for you"),
        theme::attention(now),
    ))
}

fn model_flash(state: &SessionState, now: Now) -> Option<std::time::Instant> {
    let changed = state.transcript.items().iter().rev().find(|item| {
        matches!(&item.body, bingo_sdk::ItemBody::Notice { text, .. } if text.starts_with("⇄ "))
    })?;
    let duration = std::time::Duration::from_millis(1_400);
    let elapsed = now.past(changed.started_at);
    (elapsed < duration).then(|| now.instant + duration.saturating_sub(elapsed))
}

pub fn next_change(tree: &Tree, now: Now) -> Option<std::time::Instant> {
    let slots = bingo_sdk::slot::placed(tree.viewed());
    let attention = crate::tree::count(tree, crate::tree::Wants::Attention, crate::tree::Scope::Others).is_some()
        || slots.iter().any(|placed| matches!(&placed.slot, bingo_sdk::slot::Slot::Status { tone: bingo_sdk::Tone::Attention, until, .. } if until.is_none_or(|at| at > now.wall)));
    let pulse = (now.motion && (attention || !crate::crew::active(tree).is_empty()))
        .then(|| clock::next_tick(now, theme::PULSE));
    slots
        .iter()
        .filter_map(|placed| {
            let bingo_sdk::slot::Slot::Status {
                until: Some(at), ..
            } = &placed.slot
            else {
                return None;
            };
            if *at <= now.wall {
                return None;
            }
            let left = at.duration_since(now.wall).unsigned_abs();
            let seconds = left.as_secs();
            let unit = match seconds {
                0..60 => 1,
                60..3600 => 60,
                3600..86400 => 3600,
                _ => 86400,
            };
            let boundary = std::time::Duration::from_secs(seconds / unit * unit);
            let delay = (left - boundary + std::time::Duration::from_nanos(1)).min(left);
            Some(now.instant + delay)
        })
        .chain(pulse)
        .chain(model_flash(tree.viewed(), now))
        .min()
}

/// Timed status entries use the frame clock and disappear when their time is past.
fn placed_status(state: &SessionState, now: Now) -> Vec<Span<'static>> {
    bingo_sdk::slot::placed(state)
        .into_iter()
        .filter_map(|placed| {
            let bingo_sdk::slot::Slot::Status { text, tone, until } = placed.slot else {
                return None;
            };
            let text = match until {
                Some(at) if at > now.wall => format!(
                    "{text} in {}",
                    clock::span(at.duration_since(now.wall).unsigned_abs())
                ),
                Some(_) => return None,
                None => text,
            };
            let style = match tone {
                bingo_sdk::Tone::Neutral => theme::dim(),
                bingo_sdk::Tone::Good => theme::good(),
                bingo_sdk::Tone::Bad => theme::bad(),
                bingo_sdk::Tone::Attention => theme::attention(now),
            };
            Some(Span::styled(text, style))
        })
        .collect()
}

/// What the kernel or the surface had to say, while it is being said: one at
/// a time, arriving out of dim and leaving into it (§6).
fn notice(ui: &Ui, now: Now) -> Vec<Span<'static>> {
    if ui.opening {
        return vec![Span::styled("opening a session…", theme::dim())];
    }
    let Some(notice) = ui.notice() else {
        return Vec::new();
    };
    let strength = notice.strength(now);
    let mut spans = vec![Span::styled(
        notice.text.lines().collect::<Vec<_>>().join(" · "),
        theme::fading(notice.level, strength),
    )];
    // What the refusal was about is the person's own line: it is said after
    // the reason, and never more loudly than it. The slot's own separator
    // joins the two, as it joins every other pair.
    if let Some(about) = notice.about.as_ref() {
        spans.push(Span::styled(
            about.clone(),
            theme::fading(bingo_sdk::Level::Info, strength),
        ));
    }
    spans
}

/// Where you are, how much of the window is spent, and what answers you: who
/// serves the model, which model, and how hard it is asked to think — the
/// model alone at the root.
///
/// All three are read from the frames that carry them: the summary, which the
/// kernel stamps from the choice a turn will actually run on, and the config
/// view's `thinking`, which is the one place a client reads the level
/// (ADR-0008 §4). Nothing here is a second copy of either.
fn right(tree: &Tree, site: &mut crate::component::Site<'_>, now: Now) -> Vec<Span<'static>> {
    let state = tree.viewed();
    let props = bingo_sdk::component::Props::SessionStatus(session_facts(state));
    let lines = site.draw("SessionStatus", &props, |props, _| {
        let bingo_sdk::component::Props::SessionStatus(props) = props else {
            return Vec::new();
        };
        let mut facts = Vec::new();
        if let Some(model) = &props.model {
            facts.push(Span::styled(
                model.clone(),
                if model_flash(state, now).is_some() {
                    theme::glow().patch(theme::bold())
                } else {
                    theme::text()
                },
            ));
        }
        if let Some(effort) = props.effort {
            facts.push(Span::styled(
                format!(" ∴ {}", effort.name()),
                if model_flash(state, now).is_some() {
                    theme::glow()
                } else {
                    theme::dim()
                },
            ));
        }
        if props.model_scope == Some(bingo_sdk::ChoiceScope::Session) {
            facts.push(Span::styled(" · this session", theme::ash()));
        }
        vec![Line::from(facts)]
    });
    lines.into_iter().next().unwrap_or_default().spans
}

pub(crate) fn session_facts(state: &SessionState) -> bingo_sdk::component::SessionStatusProps {
    bingo_sdk::component::SessionStatusProps {
        title: state.summary.title.clone(),
        cwd: state.summary.cwd.clone(),
        provider: state.summary.provider.clone(),
        model: state.summary.model.clone(),
        model_scope: state.summary.model_scope,
        effort: state.summary.effort.or_else(|| {
            state
                .config
                .kernel
                .get("thinking")
                .and_then(|level| serde_json::from_value::<Effort>(level.clone()).ok())
        }),
        context: state.context,
        usage: state.summary.usage,
    }
}

fn dimmed(text: String) -> Span<'static> {
    Span::styled(text, theme::dim())
}

fn join(parts: Vec<Span<'static>>) -> Vec<Span<'static>> {
    let mut out = Vec::new();
    for part in parts {
        if !out.is_empty() {
            out.push(Span::styled(" · ", theme::dim()));
        }
        out.push(part);
    }
    out
}

/// Left at the margin, right at the far edge, the middle centred in what is
/// between them — and against the left margin when there is no mode to sit
/// beside, so the hint reads where Claude Code's does.
fn place(
    left: Vec<Span<'static>>,
    middle: Vec<Span<'static>>,
    right: Vec<Span<'static>>,
    width: usize,
) -> Line<'static> {
    let (left, middle, right) = fit(left, middle, right, width);
    let slack = width.saturating_sub(cells(&left) + cells(&middle) + cells(&right));
    let before = match (left.is_empty(), middle.is_empty()) {
        (_, true) => slack,
        (true, false) => 0,
        (false, false) => slack / 2,
    };
    let mut spans = left;
    spans.push(pad(before));
    spans.extend(middle);
    spans.push(pad(slack - before));
    spans.extend(right);
    Line::from(spans)
}

/// The middle gives way first, then the mode; the place a person is in is the
/// last thing to go.
fn fit(
    left: Vec<Span<'static>>,
    middle: Vec<Span<'static>>,
    right: Vec<Span<'static>>,
    width: usize,
) -> (Vec<Span<'static>>, Vec<Span<'static>>, Vec<Span<'static>>) {
    let right = clip(right, width);
    let left = clip(left, width.saturating_sub(cells(&right)));
    let gaps = GAP * (!left.is_empty() as usize + !right.is_empty() as usize);
    let room = width.saturating_sub(cells(&left) + cells(&right) + gaps);
    (left, clip(middle, room), right)
}

fn cells(spans: &[Span<'static>]) -> usize {
    spans.iter().map(|s| s.content.width()).sum()
}

fn pad(width: usize) -> Span<'static> {
    Span::raw(" ".repeat(width))
}

/// Cut spans to `max` cells, the last one ending in an ellipsis. Nothing is
/// left of a slot with no room for even that. The quick cycle's strip is the
/// other thing this row draws, so it is cut to the row by the same rule.
pub fn clip(spans: Vec<Span<'static>>, max: usize) -> Vec<Span<'static>> {
    if cells(&spans) <= max {
        return spans;
    }
    if max <= 1 {
        return Vec::new();
    }
    let mut out: Vec<Span<'static>> = Vec::new();
    let mut used = 0;
    for span in spans {
        let room = max - 1 - used;
        let width = span.content.width();
        if width <= room {
            used += width;
            out.push(span);
            continue;
        }
        out.push(cut(&span, room));
        break;
    }
    out.push(Span::styled("…", theme::dim()));
    out
}

fn cut(span: &Span<'static>, room: usize) -> Span<'static> {
    let mut text = String::new();
    let mut used = 0;
    for c in span.content.chars() {
        let width = unicode_width::UnicodeWidthChar::width(c).unwrap_or(0);
        if used + width > room {
            break;
        }
        text.push(c);
        used += width;
    }
    Span::styled(text, span.style)
}

/// Every span of a drawn status line with the style it carries, for the tests
/// that assert where a colour landed.
#[cfg(test)]
pub fn compose(
    tree: &Tree,
    ui: &Ui,
    width: usize,
    now: Now,
) -> (Line<'static>, Option<std::ops::Range<usize>>) {
    components(tree, ui, width, now, &mut Vec::new())
}

#[cfg(test)]
pub fn line(tree: &Tree, ui: &Ui, width: usize, now: Now) -> Line<'static> {
    compose(tree, ui, width, now).0
}

#[cfg(test)]
pub fn styles(line: &Line<'static>) -> Vec<(String, ratatui::style::Style)> {
    line.spans
        .iter()
        .map(|s| (s.content.to_string(), s.style))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::*;
    use bingo_sdk::{ContextUsage, Effort, Event, ItemBody, ItemStatus};

    #[test]
    fn footer_always_names_mode_crew_and_model() {
        let (ui, now) = scene();
        let drawn = line(&solo(&state()), &ui, 108, now).to_string();
        assert!(drawn.starts_with(" ⏵⏵ default   ◆ solo"), "{drawn}");
        assert!(
            drawn.contains("▱▱▱▱▱▱▱▱ ? / ?"),
            "unknown is not zero: {drawn}"
        );
        assert!(drawn.ends_with("fake-1  "), "{drawn}");
        assert!(!drawn.contains("fake/"));
        assert!(!drawn.contains(HINT));
        assert!(
            hints(&solo(&state()), &ui, 108, now, &mut Vec::new())
                .to_string()
                .contains(HINT)
        );
    }

    #[test]
    fn model_effort_and_scope_are_summary_facts() {
        let mut state = state();
        state.summary.effort = Some(Effort::Medium);
        state.summary.model_scope = Some(bingo_sdk::ChoiceScope::Session);
        let (ui, now) = scene();
        let drawn = line(&solo(&state), &ui, 108, now).to_string();
        assert!(
            drawn.ends_with("fake-1 ∴ medium · this session  "),
            "{drawn}"
        );
        state.summary.model_scope = Some(bingo_sdk::ChoiceScope::Default);
        assert!(
            !line(&solo(&state), &ui, 108, now)
                .to_string()
                .contains("this session")
        );
    }

    #[test]
    fn old_journals_keep_their_configured_effort() {
        let state = folded(vec![frame(1, thinking_view(Some(Effort::High)))]);
        let (ui, now) = scene();
        assert!(
            line(&solo(&state), &ui, 108, now)
                .to_string()
                .contains("∴ high")
        );
    }

    #[test]
    fn a_withdrawn_wake_leaves_the_hint_row() {
        let (ui, now) = scene();
        let at = (now.wall + jiff::SignedDuration::from_secs(60)).to_string();
        let mut state = folded(vec![frame(1, pending_wake(&at))]);
        assert!(
            hints(&solo(&state), &ui, 108, now, &mut Vec::new())
                .to_string()
                .contains("wake in")
        );
        state.apply(&frame(2, wake_taken_back()));
        assert!(
            !hints(&solo(&state), &ui, 108, now, &mut Vec::new())
                .to_string()
                .contains("wake in")
        );
    }

    #[test]
    fn every_mode_uses_words_and_the_right_tone() {
        for (mode, expected, style) in [
            ("default", "⏵⏵ default", theme::mode()),
            ("acceptEdits", "⏵⏵ accept edits", theme::good()),
            ("plan", "⏸ plan", theme::crew(2)),
            ("bypassPermissions", "⏵⏵ bypass", theme::bad()),
            ("dontAsk", "⏵⏵ don't ask", theme::mode()),
        ] {
            let state = folded(vec![frame(1, permission_view(mode))]);
            let spans = left(&state);
            assert_eq!(spans[0].content, expected);
            assert_eq!(spans[0].style, style);
        }
    }

    #[test]
    fn context_bar_uses_the_window_and_turns_bad_only_above_ninety_percent() {
        for (used, fill, bad) in [
            (14_000, 1, false),
            (180_000, 7, false),
            (180_001, 7, true),
            (300_000, 8, true),
        ] {
            let context = ContextUsage {
                used,
                window: 200_000,
                trigger: 100_000,
                parts: None,
            };
            let spans = context_bar(Some(context));
            assert_eq!(spans[0].content, "▰".repeat(fill));
            assert_eq!(spans[1].content, "▱".repeat(8 - fill));
            assert_eq!(
                spans[0].style,
                if bad { theme::bad() } else { theme::presence() }
            );
            assert_eq!(spans[2].style, theme::dim());
            assert!(spans[2].content.contains(" / 200k"));
        }
    }

    #[test]
    fn narrow_footer_drops_crew_then_context_and_keeps_mode_and_model() {
        let (ui, now) = scene();
        let mut state = state();
        state.context = Some(ContextUsage {
            used: 14_000,
            window: 200_000,
            trigger: 180_000,
            parts: None,
        });
        state.summary.model = Some("a-model-with-a-long-name".into());
        for width in [108, 80, 65, 50, 40, 12, 0] {
            let drawn = line(&solo(&state), &ui, width, now);
            assert!(drawn.width() <= width, "{width}: {drawn}");
            if width >= 40 {
                assert!(drawn.to_string().contains("default"));
                assert!(drawn.to_string().contains("a-model-with-a-long-name"));
            }
        }
        let medium = line(&solo(&state), &ui, 65, now).to_string();
        assert!(!medium.contains("solo"));
        assert!(medium.contains("14k / 200k"));
        let narrow = line(&solo(&state), &ui, 50, now).to_string();
        assert!(!narrow.contains("14k / 200k"));
    }

    #[test]
    fn notices_and_repository_waits_leave_the_footer_unchanged() {
        let (mut ui, now) = scene();
        let tree = solo(&state());
        let footer = line(&tree, &ui, 108, now);
        ui.catalogs.plugins.push(bingo_sdk::CatalogEntry {
            id: "repo".into(),
            label: "repo".into(),
            meta: serde_json::json!({"state":{"kind":"waiting"}}),
        });
        assert!(
            hints(&tree, &ui, 108, now, &mut Vec::new())
                .to_string()
                .contains("1 from this repo waits for you")
        );
        ui.notify(
            bingo_sdk::Level::Info,
            "conversation was kept\nput back a.txt",
            now.instant,
        );
        assert!(
            hints(&tree, &ui, 108, now, &mut Vec::new())
                .to_string()
                .contains("conversation was kept · put back a.txt")
        );
        assert_eq!(footer, line(&tree, &ui, 108, now));
    }

    #[test]
    fn a_model_receipt_flashes_for_fourteen_hundred_milliseconds() {
        let state = folded(vec![frame(
            1,
            Event::ItemCompleted {
                item: item(
                    "change",
                    ItemStatus::Completed,
                    ItemBody::Notice {
                        level: bingo_sdk::Level::Info,
                        code: "model".into(),
                        text: "⇄ fake-1 · thinking medium".into(),
                    },
                ),
            },
        )]);
        let (_, now) = scene();
        assert!(model_flash(&state, later(now, 1_399)).is_some());
        assert!(model_flash(&state, later(now, 1_400)).is_none());
        assert!(next_change(&solo(&state), now).is_some());
    }
}
