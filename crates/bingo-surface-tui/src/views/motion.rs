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
    let Some(now) = now.filter(|now| now.motion) else {
        return style;
    };
    if !matches!(
        theme::current().colors,
        theme::Colors::True(_) | theme::Colors::Indexed(_)
    ) {
        return style;
    }
    let (other, weight) = match motion {
        Some(TextMotion::Breathe {
            with,
            period_ms,
            depth,
        }) => (
            with,
            clock::breathe(now, Duration::from_millis(*period_ms)) * depth.clamp(0.0, 1.0),
        ),
        Some(TextMotion::Fade { from, at_ms, ms }) => (from, 1.0 - clock::prog(now, *at_ms, *ms)),
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
    if !now.motion {
        return None;
    }
    if !matches!(motion, TextMotion::Spin)
        && !matches!(
            theme::current().colors,
            theme::Colors::True(_) | theme::Colors::Indexed(_)
        )
    {
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
