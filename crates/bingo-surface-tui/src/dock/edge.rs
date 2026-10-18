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
    if !state.interactions.is_empty() {
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
    if let Some(turn) = &state.turn {
        if turn.retrying.is_some() {
            return Drawing::still(vec![super::line(
                theme::blended("amber", "you", 0.45).unwrap_or_default(),
                width,
            )]);
        }
        let ink = tree
            .viewing()
            .map(|s| format!("crew{}", crate::face::face(&crate::tree::name(s)).hue()))
            .unwrap_or_else(|| "glow".into());
        return comet(width, now.past(turn.started_at), &ink, now);
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
