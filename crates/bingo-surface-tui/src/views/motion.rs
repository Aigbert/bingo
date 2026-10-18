//! A View's motion is sampled and scheduled from the same frame clock.
use crate::clock::{self, Now};
use crate::theme;
use bingo_sdk::view::TextMotion;
use ratatui::style::{Color, Style};
use std::time::{Duration, Instant};

pub fn style(style: Style, ink: Color, motion: Option<&TextMotion>, now: Option<Now>) -> Style {
    let (Some(motion), Some(now)) = (motion, now) else {
        return style;
    };
    let Some(Moving::Ink(other)) = moves(motion, now) else {
        return style;
    };
    let weight = match motion {
        TextMotion::Breathe {
            period_ms, depth, ..
        } => clock::breathe(now, Duration::from_millis(*period_ms)) * depth.clamp(0.0, 1.0),
        TextMotion::Fade { at_ms, ms, .. } => 1.0 - clock::prog(now, *at_ms, *ms),
        _ => return style,
    };
    if weight == 0.0 {
        return style;
    }
    style.patch(theme::literal(theme::ink::mix(ink, other, weight)))
}

pub fn next(motion: &TextMotion, now: Now) -> Option<Instant> {
    moves(motion, now)?;
    match motion {
        TextMotion::Spin => Some(crate::spinner::next(now)),
        TextMotion::Breathe { .. } => Some(now.instant + clock::FRAME),
        TextMotion::Fade { at_ms, ms, .. } if clock::prog(now, *at_ms, *ms) < 1.0 => {
            let until = at_ms.saturating_sub(now.wall.as_millisecond());
            Some(
                now.instant
                    + if until > 0 {
                        Duration::from_millis(until as u64)
                    } else {
                        clock::FRAME
                    },
            )
        }
        _ => None,
    }
}

enum Moving {
    Spin,
    Ink(Color),
}

fn moves(motion: &TextMotion, now: Now) -> Option<Moving> {
    if !now.motion {
        return None;
    }
    let ink = match motion {
        TextMotion::Spin => return Some(Moving::Spin),
        TextMotion::Breathe {
            with,
            period_ms,
            depth,
        } if *period_ms > 0 && *depth > 0.0 => with,
        TextMotion::Fade { from, .. } => from,
        _ => return None,
    };
    if !matches!(
        theme::current().colors,
        theme::Colors::True(_) | theme::Colors::Indexed(_)
    ) {
        return None;
    }
    theme::palette().named(ink).map(Moving::Ink)
}
