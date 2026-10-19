//! Ambient facts and the focused site's pairs share the last row.
use crate::{
    clock::{self, Now},
    component::Site,
    layout::{Layout, grid},
    theme,
    tree::Tree,
    ui::Ui,
    views::Drawing,
};
use bingo_sdk::component::{Props, SessionModeProps};
use ratatui::text::{Line, Span};
use std::time::Duration;
mod toast;
mod view;

pub fn draw(tree: &Tree, ui: &Ui, frame: &mut Layout, width: usize, now: Now) -> Drawing {
    let available = width.saturating_sub(grid::TX + grid::RM);
    let is_toast = crate::toast::current(ui, tree.viewed(), now).is_some();
    let facts = if is_toast {
        Vec::new()
    } else {
        ambient(tree, ui, frame, available, now)
    };
    // The keys come before every fact but the first, as `footerLine()` has
    // them; the facts after it are cut short with an ellipsis.
    let lead = facts
        .iter()
        .filter_map(|fact| fact.lines.first())
        .map(Line::width)
        .find(|width| *width > 0)
        .map_or(0, |width| width + 3);
    let right = crate::keybar::draw(tree, ui, frame, available.saturating_sub(lead), now);
    let right_width = right.lines.first().map_or(0, Line::width);
    let room = available.saturating_sub(right_width + usize::from(right_width > 0) * 3);
    let ambient = fit(facts, room);
    let mut left = toast::draw(tree.viewed(), ui, frame, room, now).unwrap_or(ambient);
    let line = left.lines.pop().unwrap_or_default();
    let mut spans = vec![Span::raw(" ".repeat(grid::TX.min(width)))];
    spans.extend(line.spans);
    if let Some(right_line) = right.lines.into_iter().next() {
        let used: usize = spans.iter().map(Span::width).sum();
        let style = spans
            .last()
            .zip(right_line.spans.first())
            .filter(|(a, b)| a.style == b.style)
            .map(|(a, _)| a.style)
            .unwrap_or_default();
        spans.push(Span::styled(
            " ".repeat(width.saturating_sub(grid::RM + right_line.width() + used)),
            style,
        ));
        spans.extend(right_line.spans);
    }
    Drawing {
        lines: vec![Line::from(spans)],
        next: left.next.into_iter().chain(right.next).min(),
    }
}

fn ambient(tree: &Tree, ui: &Ui, frame: &mut Layout, room: usize, now: Now) -> Vec<Drawing> {
    crate::layout::count::band(3);
    let mut site = Site {
        state: tree.viewed(),
        memo: &ui.placed.pulled,
        width: room,
        pulls: &mut frame.pulls,
        now,
    };
    let lead = needs_you(tree);
    let mut parts = match view::draw(tree) {
        Some(whose) if bingo_collab::room::is_room(&tree.viewed().summary) => vec![whose, lead],
        Some(whose) => vec![whose, model(tree.viewed(), &mut site), lead],
        None => vec![lead, model(tree.viewed(), &mut site)],
    };
    parts.push(mode(tree.viewed(), &mut site));
    parts.push(context(tree.viewed()));
    if tree.viewing().is_none() && tree.needers().next().is_none() {
        parts.push(crew(tree, now));
    }
    parts.extend(slots(tree.viewed(), now));
    parts
}

fn model(state: &bingo_sdk::SessionState, site: &mut Site<'_>) -> Drawing {
    site.draw(
        "SessionStatus",
        &Props::SessionStatus(session_facts(state)),
        |props, _| {
            let Props::SessionStatus(props) = props else {
                return Vec::new();
            };
            let mut spans = Vec::new();
            if let Some(model) = &props.model {
                let model = model
                    .rsplit('/')
                    .find(|part| !part.is_empty())
                    .unwrap_or(model);
                spans.push(Span::styled(model.to_owned(), ink("muted")));
                if let Some(effort) = props.effort {
                    spans.push(Span::styled(format!(" {}", effort.name()), ink("dim")));
                }
            }
            vec![Line::from(spans)]
        },
    )
}
fn mode(state: &bingo_sdk::SessionState, site: &mut Site<'_>) -> Drawing {
    let mode = crate::mode::current(state);
    if mode.value == "default" {
        return Drawing::default();
    }
    site.draw(
        "SessionMode",
        &Props::SessionMode(SessionModeProps {
            modes: vec![mode.label.clone()],
        }),
        |props, _| {
            let Props::SessionMode(props) = props else {
                return Vec::new();
            };
            vec![Line::from(Span::styled(
                props.modes.join(" · "),
                ink(mode.ink()),
            ))]
        },
    )
}
fn context(state: &bingo_sdk::SessionState) -> Drawing {
    let Some(context) = state.context else {
        return Drawing::default();
    };
    let share = context.used as f64 / context.window.max(1) as f64;
    if share < 0.7 {
        return Drawing::default();
    }
    Drawing::still(vec![Line::from(Span::styled(
        format!("context {}%", (share * 100.0).round()),
        ink(if share >= 0.8 { "amber" } else { "muted" }),
    ))])
}
/// The working or asking descendants of the session on screen, in tree order.
pub(crate) fn active(tree: &Tree) -> Vec<&bingo_sdk::SessionState> {
    let under = crate::tree::covered(tree, crate::tree::Scope::Under(tree.view()));
    tree.sessions()
        .filter(|state| under.contains(&&state.summary.id))
        .filter(|state| {
            matches!(
                bingo_collab::lifecycle::Lifecycle::of_state(state),
                Some(
                    bingo_collab::lifecycle::Lifecycle::Working
                        | bingo_collab::lifecycle::Lifecycle::Asking
                )
            )
        })
        .collect()
}
fn crew(tree: &Tree, now: Now) -> Drawing {
    let agents: Vec<_> = active(tree)
        .into_iter()
        .filter(|s| s.busy() && s.interactions.is_empty())
        .collect();
    if agents.is_empty() {
        return Drawing::default();
    }
    let faces = crate::face::Faces::of(tree);
    let mut spans: Vec<_> = agents
        .iter()
        .take(4)
        .enumerate()
        .map(|(i, state)| {
            let name = faces.ink(&state.summary.id);
            let shifted = Now {
                wall: now.wall + jiff::SignedDuration::from_millis(i as i64 * 450),
                ..now
            };
            Span::styled(
                theme::glyphs::get("lamp"),
                theme::blended(
                    name,
                    "bg",
                    0.35 * clock::breathe(shifted, Duration::from_millis(1800)),
                )
                .unwrap_or_default(),
            )
        })
        .collect();
    spans.push(Span::styled(
        format!(
            " {} agent{}",
            agents.len(),
            if agents.len() == 1 { "" } else { "s" }
        ),
        ink("muted"),
    ));
    Drawing {
        lines: vec![Line::from(spans)],
        next: now.motion.then_some(now.instant + clock::FRAME),
    }
}
fn needs_you(tree: &Tree) -> Drawing {
    use crate::tree::{Scope, Wants};
    let rows = crate::tree::matching(tree, Wants::Attention, Scope::Others);
    let count = rows.len();
    if count == 0 {
        return Drawing::default();
    }
    let mut spans = vec![Span::styled(
        format!("{} ", theme::glyphs::get("pin")),
        ink("amberGlow"),
    )];
    if count > 2 {
        spans.push(Span::styled(count.to_string(), ink("amber")));
    } else {
        for (index, row) in rows.into_iter().enumerate() {
            if index > 0 {
                spans.push(Span::styled(", ", ink("amber")));
            }
            let (name, color) = if row.session == tree.root_id() {
                ("bingo".to_owned(), "glow".to_owned())
            } else {
                let color = crate::face::Faces::of(tree).ink(row.session).to_owned();
                (row.name, color)
            };
            spans.push(Span::styled(name, ink(&color)));
        }
    }
    spans.push(Span::styled(
        if count == 1 {
            " needs you"
        } else {
            " need you"
        },
        ink("amber"),
    ));
    Drawing::still(vec![Line::from(spans)])
}

/// The countdown reads seconds under an hour and minutes past it
/// (`bingo_sdk::remaining`), so it changes on the next whole unit.
fn countdown_change(remaining: Duration) -> Duration {
    let unit = if remaining < Duration::from_secs(3600) {
        1
    } else {
        60
    };
    let seconds = remaining.as_secs();
    (remaining - Duration::from_secs(seconds / unit * unit) + Duration::from_nanos(1))
        .min(remaining)
}

/// `"{text} in {remaining}"` until `at`, and the instant its words change;
/// nothing once `at` has passed.
fn countdown(text: &str, at: jiff::Timestamp, now: Now) -> Option<(String, std::time::Instant)> {
    let left = at.duration_since(now.wall);
    if !left.is_positive() {
        return None;
    }
    let left = left.unsigned_abs();
    let millis = i64::try_from(left.as_millis()).unwrap_or(i64::MAX);
    Some((
        format!("{text} in {}", bingo_sdk::remaining(millis)),
        now.instant + countdown_change(left),
    ))
}

fn slots(state: &bingo_sdk::SessionState, now: Now) -> Vec<Drawing> {
    bingo_sdk::slot::placed(state)
        .into_iter()
        .filter_map(|placed| {
            let bingo_sdk::slot::Slot::Status { text, until, .. } = placed.slot else {
                return None;
            };
            let (text, next) = match until {
                Some(at) => countdown(&text, at, now).map(|(text, next)| (text, Some(next)))?,
                None => (text, None),
            };
            Some(Drawing {
                lines: vec![Line::from(Span::styled(text, ink("muted")))],
                next,
            })
        })
        .collect()
}

/// The facts in order, a separator between each, cut with an ellipsis
/// where the row ends (`fitSegs()`).
fn fit(parts: Vec<Drawing>, width: usize) -> Drawing {
    let next = parts.iter().filter_map(|part| part.next).min();
    let mut line = Line::default();
    for part in parts {
        let Some(part) = part.lines.into_iter().next().filter(|l| l.width() > 0) else {
            continue;
        };
        if line.width() > 0 {
            line.spans
                .push(Span::styled(theme::glyphs::get("sep"), ink("faint")));
        }
        line.spans.extend(part.spans);
    }
    Drawing {
        lines: vec![grid::fit(line, width)],
        next,
    }
}

pub(super) fn ink(name: &str) -> ratatui::style::Style {
    theme::foreground(name).unwrap_or_default()
}

pub(crate) fn session_facts(
    state: &bingo_sdk::SessionState,
) -> bingo_sdk::component::SessionStatusProps {
    bingo_sdk::component::SessionStatusProps {
        receipt: None,
        title: state.summary.title.clone(),
        cwd: state.summary.cwd.clone(),
        provider: state.summary.provider.clone(),
        model: state.summary.model.clone(),
        model_scope: state.summary.model_scope,
        effort: state.summary.effort,
        context: state.context,
        usage: state.summary.usage,
    }
}

#[cfg(test)]
pub(crate) mod tests;
