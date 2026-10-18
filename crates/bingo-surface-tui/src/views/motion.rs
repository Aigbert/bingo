//! A View's motion is sampled and scheduled from the same frame clock.
use crate::clock::{self, Now};
use crate::theme;
use bingo_sdk::view::TextMotion;
use ratatui::style::Style;
use std::time::{Duration, Instant};

pub fn style(
    style: Style,
    ink: ratatui::style::Color,
    motion: Option<&TextMotion>,
    now: Option<Now>,
) -> Style {
    let (Some(motion), Some(now)) = (motion, now) else {
        return style;
    };
    if !moves(motion, now) {
        return style;
    }
    let (other, weight) = match motion {
        TextMotion::Breathe {
            with,
            period_ms,
            depth,
        } => (
            with,
            clock::breathe(now, Duration::from_millis(*period_ms)) * depth.clamp(0.0, 1.0),
        ),
        TextMotion::Fade { from, at_ms, ms } => (from, 1.0 - clock::prog(now, *at_ms, *ms)),
        _ => return style,
    };
    if weight == 0.0 {
        return style;
    }
    let Some(other) = theme::palette().named(other) else {
        return style;
    };
    style.patch(theme::literal(theme::ink::mix(ink, other, weight)))
}

pub fn next(motion: &TextMotion, now: Now) -> Option<Instant> {
    if !moves(motion, now) {
        return None;
    }
    match motion {
        TextMotion::Spin => Some(crate::spinner::next(now)),
        TextMotion::Breathe {
            period_ms, depth, ..
        } if *period_ms > 0 && *depth > 0.0 => Some(now.instant + clock::FRAME),
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

fn moves(motion: &TextMotion, now: Now) -> bool {
    if !now.motion {
        return false;
    }
    match motion {
        TextMotion::Spin => true,
        TextMotion::Breathe { with: ink, .. } | TextMotion::Fade { from: ink, .. } => {
            matches!(
                theme::current().colors,
                theme::Colors::True(_) | theme::Colors::Indexed(_)
            ) && theme::palette().named(ink).is_some()
        }
        TextMotion::Other(_) => false,
    }
}
