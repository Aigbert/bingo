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

pub fn draw(tree: &Tree, ui: &Ui, frame: &mut Layout, width: usize, now: Now) -> Drawing {
    if width == 0 {
        return Drawing::default();
    }
    let available = width.saturating_sub(grid::TX + grid::RM);
    let is_toast = crate::toast::current(ui, tree.viewed(), now).is_some();
    let ambient = if is_toast {
        Drawing::default()
    } else {
        ambient(tree, ui, frame, available, now)
    };
    let reserved = if is_toast {
        0
    } else {
        ambient.lines.first().map_or(0, Line::width)
    };
    let right = crate::keybar::draw(
        tree,
        ui,
        frame,
        available.saturating_sub(reserved + usize::from(reserved > 0) * 3),
        now,
    );
    let right_width = right.lines.first().map_or(0, Line::width);
    let room = available.saturating_sub(right_width + usize::from(right_width > 0) * 3);
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

fn ambient(tree: &Tree, ui: &Ui, frame: &mut Layout, room: usize, now: Now) -> Drawing {
    crate::layout::count::band(3);
    if room == 0 {
        return Drawing::default();
    }
    let mut site = Site {
        state: tree.viewed(),
        memo: &ui.placed.pulled,
        width: room,
        pulls: &mut frame.pulls,
        now,
    };
    let lead = needs_you(tree);
    let needs = !lead.lines.is_empty();
    let mut parts = vec![
        lead,
        model(tree.viewed(), &mut site),
        mode(tree.viewed(), &mut site),
    ];
    parts.push(context(tree.viewed()));
    if !needs {
        parts.push(crew(tree, now));
    }
    parts.extend(slots(tree.viewed(), now));
    fit(parts, room)
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
fn crew(tree: &Tree, now: Now) -> Drawing {
    if tree.viewing().is_some() {
        return Drawing::default();
    }
    let agents: Vec<_> = crate::crew::active(tree)
        .into_iter()
        .filter(|s| s.busy() && s.interactions.is_empty())
        .collect();
    if agents.is_empty() {
        return Drawing::default();
    }
    let mut spans: Vec<_> = agents
        .iter()
        .take(4)
        .enumerate()
        .map(|(i, state)| {
            let name = format!("crew{}", crate::face::face(&crate::tree::name(state)).hue());
            let shifted = Now {
                wall: now.wall + jiff::SignedDuration::from_millis(i as i64 * 450),
                ..now
            };
            Span::styled(
                theme::glyphs::get("lamp"),
                theme::blended(
                    &name,
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
    let Some(count) = crate::tree::count(tree, Wants::Attention, Scope::Others) else {
        return Drawing::default();
    };
    let mut spans = vec![Span::styled(
        format!("{} ", theme::glyphs::get("pin")),
        ink("amberGlow"),
    )];
    if count > 2 {
        spans.push(Span::styled(count.to_string(), ink("amber")));
    } else {
        for (index, row) in tree
            .rows()
            .into_iter()
            .filter(|row| row.attention && row.session != tree.view())
            .enumerate()
        {
            if index > 0 {
                spans.push(Span::styled(", ", ink("amber")));
            }
            let color = if row.session == tree.root_id() {
                "glow".to_owned()
            } else {
                format!("crew{}", crate::face::face(&row.name).hue())
            };
            spans.push(Span::styled(row.name, ink(&color)));
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

fn countdown_change(remaining: Duration) -> Duration {
    let seconds = remaining.as_secs();
    let unit = match seconds {
        0..60 => 1,
        60..3600 => 60,
        3600..86400 => 3600,
        _ => 86400,
    };
    (remaining - Duration::from_secs(seconds / unit * unit) + Duration::from_nanos(1))
        .min(remaining)
}

fn slots(state: &bingo_sdk::SessionState, now: Now) -> Vec<Drawing> {
    bingo_sdk::slot::placed(state)
        .into_iter()
        .filter_map(|placed| {
            let bingo_sdk::slot::Slot::Status { text, until, .. } = placed.slot else {
                return None;
            };
            let (text, next) = match until {
                Some(at) if at > now.wall => (
                    format!(
                        "{text} in {}",
                        clock::span(at.duration_since(now.wall).unsigned_abs())
                    ),
                    Some(
                        now.instant + countdown_change(at.duration_since(now.wall).unsigned_abs()),
                    ),
                ),
                Some(_) => return None,
                None => (text, None),
            };
            Some(Drawing {
                lines: vec![Line::from(Span::styled(text, ink("muted")))],
                next,
            })
        })
        .collect()
}

fn fit(parts: Vec<Drawing>, width: usize) -> Drawing {
    let mut out = Drawing::default();
    let mut line = Line::default();
    for part in parts {
        let Some(next) = part.lines.into_iter().next().filter(|l| l.width() > 0) else {
            continue;
        };
        let gap = usize::from(line.width() > 0) * 3;
        if line.width() + gap + next.width() > width {
            if line.width() == 0 {
                line = grid::fit(next, width);
                out.next = part.next;
            }
            break;
        }
        if gap > 0 {
            line.spans
                .push(Span::styled(theme::glyphs::get("sep"), ink("faint")));
        }
        line.spans.extend(next.spans);
        out.next = out.next.into_iter().chain(part.next).min();
    }
    out.lines.push(line);
    out
}
pub(super) fn ink(name: &str) -> ratatui::style::Style {
    theme::foreground(name).unwrap_or_default()
}

#[cfg(test)]
mod tests;

pub(crate) fn session_facts(
    state: &bingo_sdk::SessionState,
) -> bingo_sdk::component::SessionStatusProps {
    bingo_sdk::component::SessionStatusProps {
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
pub fn line(
    tree: &crate::tree::Tree,
    ui: &crate::ui::Ui,
    width: usize,
    now: crate::clock::Now,
) -> ratatui::text::Line<'static> {
    let mut frame = crate::layout::Layout::default();
    crate::footer::draw(tree, ui, &mut frame, width, now)
        .lines
        .into_iter()
        .next()
        .unwrap_or_default()
}
