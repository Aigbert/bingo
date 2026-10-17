//! A running step's spinner. Its frame is read off the wall clock, so every
//! running row turns on the same beat without anybody holding an origin, and
//! where nothing may move it stands still as the waiting glyph.

use std::sync::OnceLock;
use std::time::{Duration, Instant};

use crate::clock::{self, Now};
use crate::theme;

/// The fastest beat a multiplexer passes on whole: tmux coalesces redraws
/// below this, and a spinner it skips frames of stutters.
const MULTIPLEXED: Duration = Duration::from_millis(120);

/// The beat a spinner turns at: the table's own, slowed under a multiplexer.
pub fn period(spin: Duration, multiplexed: bool) -> Duration {
    match multiplexed {
        true => spin.max(MULTIPLEXED),
        false => spin,
    }
}

/// The beat this process's spinner turns at.
pub fn beat() -> Duration {
    period(theme::glyphs().chrome.spin, multiplexed())
}

fn multiplexed() -> bool {
    static TMUX: OnceLock<bool> = OnceLock::new();
    *TMUX.get_or_init(|| std::env::var_os("TMUX").is_some())
}

/// The glyph a running step wears on this frame.
pub fn glyph(now: Now) -> &'static str {
    let chrome = theme::glyphs().chrome;
    if !now.motion || chrome.spinner.is_empty() {
        return chrome.steps[2];
    }
    chrome.spinner[frame(now, beat(), chrome.spinner.len())]
}

/// Which of `frames` the wall clock is on, one per `beat`.
fn frame(now: Now, beat: Duration, frames: usize) -> usize {
    let beat = i128::try_from(beat.as_nanos()).unwrap_or(i128::MAX).max(1);
    let beats = now.wall.as_nanosecond().div_euclid(beat);
    usize::try_from(beats.rem_euclid(frames.max(1) as i128)).unwrap_or_default()
}

/// When the spinner next turns.
pub fn next(now: Now) -> Instant {
    clock::next_tick(now, beat())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::{scene, still};

    #[test]
    fn a_multiplexer_slows_the_beat_and_never_speeds_it() {
        let ms = Duration::from_millis;
        assert_eq!(period(ms(80), false), ms(80));
        assert_eq!(period(ms(80), true), ms(120));
        assert_eq!(period(ms(130), true), ms(130));
    }

    #[test]
    fn two_rows_on_one_frame_wear_one_glyph_and_the_next_beat_turns_it() {
        let now = scene().1;
        let beat = Duration::from_millis(80);
        let later = Now {
            wall: now.wall + jiff::SignedDuration::from_millis(80),
            ..now
        };
        assert_eq!(frame(now, beat, 10), frame(now, beat, 10));
        assert_eq!((frame(now, beat, 10) + 1) % 10, frame(later, beat, 10));
    }

    #[test]
    fn where_nothing_moves_it_waits() {
        let now = still(scene().1);
        assert_eq!(glyph(now), theme::glyphs().chrome.steps[2]);
        crate::theme::with(crate::painted::ascii(), || assert_eq!(glyph(now), "o"));
    }
}
