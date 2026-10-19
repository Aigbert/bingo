use crate::{
    clock::{self, Now},
    mode, theme,
    tree::Tree,
    ui::Ui,
    views::Drawing,
};
use ratatui::text::{Line, Span};
use std::time::Duration;

pub fn draw(tree: &Tree, ui: &Ui, width: usize, now: Now) -> Drawing {
    let state = tree.viewed();
    if state.interactions.iter().any(cautioned) {
        return Drawing::still(vec![super::line(theme::bad(), width)]);
    }
    // A device code waits on the browser, not on an answer typed here: its
    // sheet is the whole signal, so the edge keeps its quiet.
    if state.interactions.iter().any(|open| {
        !matches!(
            open.kind,
            bingo_sdk::InteractionKind::Login {
                flow: bingo_sdk::LoginFlow::Device { .. },
                ..
            }
        )
    }) {
        let ink = theme::blended(
            "amberGlow",
            "you",
            0.35 * clock::breathe(now, Duration::from_millis(1400)),
        )
        .unwrap_or_default();
        return sampled(
            super::line(ink, width),
            now.motion.then_some(now.instant + clock::FRAME),
        );
    }
    if let Some(drawing) = written_in(tree, width, now) {
        return drawing;
    }
    if let Some(turn) = &state.turn {
        if turn.retrying.is_some() {
            return Drawing::still(vec![super::line(
                theme::blended("amber", "you", 0.45).unwrap_or_default(),
                width,
            )]);
        }
        let ink = crate::face::Faces::of(tree).view_ink(&state.summary.id);
        return comet(width, now.past(turn.started_at), ink, now);
    }
    if let Some(drawing) = walked_in(tree, ui, width, now) {
        return drawing;
    }
    if let Some(age) = mode::changed(ui, now)
        && state
            .last_turn
            .as_ref()
            .is_none_or(|turn| age <= now.past(turn.ended_at))
    {
        return flash(width, mode::current(state).flash_ink(), age, 900, now);
    }
    if let Some(shell) = crate::transcript::ran::latest(state)
        && let Some(end) = shell.completed_at
    {
        let ink = if matches!(shell.body, bingo_sdk::ItemBody::Shell { exit: Some(0), .. }) {
            "good"
        } else {
            "bad"
        };
        return flash(width, ink, now.past(end), 1100, now);
    }
    if let Some(turn) = &state.last_turn {
        let ink = match turn.status {
            bingo_sdk::TurnStatus::Completed => "good",
            bingo_sdk::TurnStatus::Failed { .. } => "bad",
            _ => return Drawing::still(vec![Line::default()]),
        };
        return flash(width, ink, now.past(turn.ended_at), 1100, now);
    }
    Drawing::still(vec![Line::default()])
}

/// An ask that could lose work holds the edge still, in the colour of harm.
fn cautioned(open: &bingo_sdk::Interaction) -> bool {
    matches!(
        open.kind,
        bingo_sdk::InteractionKind::Permission {
            caution: Some(_),
            ..
        }
    )
}

/// Someone writing in the room on view runs the comet in their ink, from
/// when their turn began (`edgeLine`'s `typing`).
fn written_in(tree: &Tree, width: usize, now: Now) -> Option<Drawing> {
    let state = tree.viewed();
    if state.summary.driver != bingo_sdk::Driver::Log {
        return None;
    }
    let faces = crate::face::Faces::of(tree);
    let name = crate::transcript::room::writing(state, &faces)
        .into_iter()
        .next()?;
    let turn = tree.state(&faces.teammate(&name)?.id)?.turn.as_ref()?;
    Some(comet(
        width,
        now.past(turn.started_at),
        faces.who(&name),
        now,
    ))
}

/// Walking into a session flashes the edge in that session's ink, until
/// anything the edge says later takes its place (`enterView`).
fn walked_in(tree: &Tree, ui: &Ui, width: usize, now: Now) -> Option<Drawing> {
    let age = ui
        .switched
        .map(|at| now.since(at))
        .filter(|age| *age < WALK_IN)?;
    let state = tree.viewed();
    let said_since = |at: Option<Duration>| at.is_some_and(|since| since < age);
    let later = said_since(mode::changed(ui, now))
        || said_since(state.last_turn.as_ref().map(|turn| now.past(turn.ended_at)))
        || said_since(
            crate::transcript::ran::latest(state)
                .and_then(|shell| shell.completed_at)
                .map(|end| now.past(end)),
        );
    let ink = crate::face::Faces::of(tree).ink(&state.summary.id);
    (!later).then(|| flash(width, ink, age, WALK_IN.as_millis() as u64, now))
}

/// As long as a change of mode flashes: walking in is a change of place.
const WALK_IN: Duration = Duration::from_millis(900);

fn sampled(line: Line<'static>, next: Option<std::time::Instant>) -> Drawing {
    Drawing {
        lines: vec![line],
        next,
    }
}

fn flash(width: usize, ink: &str, age: Duration, ms: u64, now: Now) -> Drawing {
    let length = Duration::from_millis(ms);
    if age >= length {
        return Drawing::still(vec![Line::default()]);
    }
    let k = if now.motion {
        f64::from(clock::ease_in_out(age.as_secs_f32() / length.as_secs_f32()))
    } else {
        0.2
    };
    let line = super::line(theme::blended(ink, "you", k).unwrap_or_default(), width);
    let end = now.instant + length - age;
    sampled(
        line,
        Some(if now.motion {
            (now.instant + clock::FRAME).min(end)
        } else {
            end
        }),
    )
}

pub(super) fn comet(width: usize, age: Duration, ink: &str, now: Now) -> Drawing {
    if !now.motion {
        return Drawing::still(vec![super::line(
            theme::blended("faint", ink, 0.4).unwrap_or_default(),
            width,
        )]);
    }
    let length = (width as f64 * 0.22).round().min(22.0);
    let phase = (age.as_secs_f64() % 2.2) / 2.2;
    let head =
        (f64::from(clock::ease_in_out(phase as f32)) * (width as f64 + length)).round() - 1.0;
    let spans = (0..width)
        .map(|x| {
            let distance = head - x as f64;
            let weight = if distance >= 0.0 && distance < length {
                (1.0 - distance / length).powf(1.7)
            } else {
                0.0
            };
            Span::styled(
                theme::glyphs::get("edge"),
                theme::blended("faint", ink, weight).unwrap_or_default(),
            )
        })
        .collect::<Vec<_>>();
    sampled(Line::from(spans), Some(now.instant + clock::FRAME))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::*;
    use bingo_sdk::Event;

    fn ink_of(drawing: &Drawing) -> ratatui::style::Style {
        drawing.lines[0].spans[0].style
    }

    #[test]
    fn a_teammate_working_runs_the_comet_in_its_own_ink() {
        let mut tree = spawned_tree(busy_child("reviewer"));
        tree.show(&child_id());
        let (ui, now) = scene();
        let ink = crate::face::Faces::of(&tree).view_ink(&child_id());
        assert_ne!(ink, "glow");
        let drawn = draw(&tree, &ui, 40, still(now));
        assert_eq!(
            ink_of(&drawn),
            theme::blended("faint", ink, 0.4).unwrap_or_default()
        );
    }

    #[test]
    fn walking_in_flashes_the_edge_in_the_session_ink_for_900_ms() {
        let idle = vec![child_frame(
            1,
            Event::SessionUpdated {
                summary: child_summary("reviewer"),
            },
        )];
        let mut tree = spawned_tree(idle);
        tree.show(&child_id());
        let (mut ui, now) = scene();
        assert_eq!(draw(&tree, &ui, 40, now).lines[0], Line::default());
        ui.switched = Some(now.instant);
        let ink = crate::face::Faces::of(&tree).ink(&child_id());
        let drawn = draw(&tree, &ui, 40, still(now));
        assert_eq!(
            ink_of(&drawn),
            theme::blended(ink, "you", 0.2).unwrap_or_default()
        );
        assert_eq!(drawn.next, Some(now.instant + WALK_IN));
        let after = draw(&tree, &ui, 40, later(now, 900));
        assert_eq!(after.lines[0], Line::default());
    }
}
