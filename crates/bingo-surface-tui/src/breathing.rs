//! A running step's bullet: `presence` breathing to its glow. The phase is
//! read off the wall clock, so every running row breathes on the same beat
//! without anybody holding an origin, and where nothing may move it stands
//! still in `presence`.

use std::sync::OnceLock;
use std::time::{Duration, Instant};

use ratatui::style::Style;

use crate::clock::{self, Now};
use crate::theme;

/// One breath of a running step's bullet (§6).
const BREATH: Duration = Duration::from_millis(1200);
/// How often a breath is drawn again: often enough for each of the steps a
/// pulse is drawn in to land.
const BEAT: Duration = Duration::from_millis(80);
/// The fastest beat a multiplexer passes on whole: tmux coalesces redraws
/// below this, and a breath it skips frames of stutters.
const MULTIPLEXED: Duration = Duration::from_millis(120);

/// The beat a breath is drawn again at, slowed under a multiplexer.
pub fn period(beat: Duration, multiplexed: bool) -> Duration {
    match multiplexed {
        true => beat.max(MULTIPLEXED),
        false => beat,
    }
}

fn multiplexed() -> bool {
    static TMUX: OnceLock<bool> = OnceLock::new();
    *TMUX.get_or_init(|| std::env::var_os("TMUX").is_some())
}

/// What a running step's bullet is drawn in on the frame at `now`.
pub fn style(now: Now) -> Style {
    match now.motion {
        true => theme::pulse(clock::breath(now, BREATH)),
        false => theme::presence(),
    }
}

/// When the breath is next drawn again.
pub fn next(now: Now) -> Instant {
    clock::next_tick(now, period(BEAT, multiplexed()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::{later, scene, still};

    #[test]
    fn a_multiplexer_slows_the_beat_and_never_speeds_it() {
        let ms = Duration::from_millis;
        assert_eq!(period(ms(80), false), ms(80));
        assert_eq!(period(ms(80), true), ms(120));
        assert_eq!(period(ms(130), true), ms(130));
    }

    #[test]
    fn half_a_breath_on_it_is_drawn_in_another_ink() {
        let now = scene().1;
        let half = i64::try_from(BREATH.as_millis() / 2).unwrap_or(600);
        crate::theme::with(crate::painted::truecolor(), || {
            assert_ne!(style(now), style(later(now, half)));
        });
    }

    #[test]
    fn where_nothing_moves_it_stands_in_presence() {
        let now = still(scene().1);
        assert_eq!(style(now), theme::presence());
    }
}
