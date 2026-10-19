//! An ask ignores keys for a moment after it opens, so a key meant for the
//! composer does not answer it; the picked row fades in over that moment.
use crate::clock::{self, Now};
use jiff::Timestamp;
use std::time::{Duration, Instant};

pub const GUARD: Duration = Duration::from_millis(400);

/// Whether a key at `now` counts for an ask that opened at `opened`.
pub fn open(opened: Timestamp, now: Timestamp) -> bool {
    elapsed(opened, now) >= GUARD
}

/// How far the picked row has come from the dock's ground: 0 when the ask
/// opens, 1 once keys count. A still surface draws the resting frame.
pub fn fade(opened: Timestamp, now: Now) -> f64 {
    if !now.motion {
        return 1.0;
    }
    (elapsed(opened, now.wall).as_secs_f64() / GUARD.as_secs_f64()).min(1.0)
}

/// When the fade next changes, while it is still running.
pub fn next(opened: Timestamp, now: Now) -> Option<Instant> {
    let left = GUARD.checked_sub(elapsed(opened, now.wall))?;
    (now.motion && !left.is_zero()).then(|| (now.instant + clock::FRAME).min(now.instant + left))
}

fn elapsed(opened: Timestamp, now: Timestamp) -> Duration {
    Duration::from_millis(u64::try_from((now - opened).get_milliseconds()).unwrap_or(0))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn at(ms: i64) -> Timestamp {
        Timestamp::from_millisecond(1_000_000 + ms).unwrap()
    }

    #[test]
    fn the_guard_swallows_a_key_at_399_ms_and_takes_it_at_400() {
        assert!(!open(at(0), at(399)));
        assert!(open(at(0), at(400)));
        assert!(!open(at(0), at(-5)));
    }

    #[test]
    fn the_fade_runs_with_the_guard_and_wakes_until_it_ends() {
        let (_, now) = crate::test_support::scene();
        let moving = |ms| Now {
            wall: at(ms),
            motion: true,
            ..now
        };
        assert_eq!(fade(at(0), moving(0)), 0.0);
        assert_eq!(fade(at(0), moving(200)), 0.5);
        assert_eq!(fade(at(0), moving(900)), 1.0);
        assert!(next(at(0), moving(200)).is_some());
        assert!(next(at(0), moving(400)).is_none());
        let still = Now {
            motion: false,
            ..moving(0)
        };
        assert_eq!(fade(at(0), still), 1.0);
        assert!(next(at(0), still).is_none());
    }
}
