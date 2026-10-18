//! A View's motion is sampled and scheduled from the same frame clock.
use crate::clock::{self, Now};
use crate::theme;
use bingo_sdk::view::TextMotion;
use ratatui::style::Style;
use std::time::{Duration, Instant};

pub fn style(style: Style, motion: Option<&TextMotion>, now: Option<Now>) -> Style {
    let Some(now) = now.filter(|now| now.motion) else {
        return style;
    };
    let Some(motion) = motion else {
        return style;
    };
    match motion {
        TextMotion::Breathe {
            with,
            period_ms,
            depth,
        } => {
            let weight =
                clock::breathe(now, Duration::from_millis(*period_ms)) * depth.clamp(0.0, 1.0);
            theme::blend_style(style, with, weight).unwrap_or(style)
        }
        TextMotion::Fade { from, at_ms, ms } => {
            theme::blend_style(style, from, 1.0 - clock::prog(now, *at_ms, *ms)).unwrap_or(style)
        }
        TextMotion::Spin | TextMotion::Unknown => style,
    }
}

pub fn next(motion: &TextMotion, now: Now) -> Option<Instant> {
    if !now.motion {
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

#[derive(Clone, Copy)]
struct Pass {
    now: Now,
    next: Option<Instant>,
}
thread_local! {
    static PASS: std::cell::Cell<Option<Pass>> = const { std::cell::Cell::new(None) };
}

pub(crate) fn now() -> Option<Now> {
    PASS.get().map(|pass| pass.now)
}

pub(crate) fn observe(view: &bingo_sdk::View, now: Option<Now>) {
    let next = now.and_then(|now| super::next_change(view, now));
    if let Some(mut pass) = PASS.get() {
        pass.next = pass.next.into_iter().chain(next).min();
        PASS.set(Some(pass));
    }
}

/// A memo retains the deadline of the drawing it retains, including its last moving frame.
pub(crate) fn track<R>(now: Now, draw: impl FnOnce() -> R) -> (R, Option<Instant>) {
    let previous = PASS.replace(Some(Pass { now, next: None }));
    let restore = Restore(previous);
    let result = draw();
    let next = PASS.get().and_then(|pass| pass.next);
    drop(restore);
    (result, next)
}
struct Restore(Option<Pass>);
impl Drop for Restore {
    fn drop(&mut self) {
        let next = PASS.get().and_then(|pass| pass.next);
        PASS.set(self.0.map(|mut pass| {
            pass.next = pass.next.into_iter().chain(next).min();
            pass
        }));
    }
}
