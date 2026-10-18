//! The wordmark entrance, a pure function of elapsed seconds.

use std::time::Instant;

use crate::clock::Now;

pub const END: f32 = 2.4;

/// The opening as the surface holds it while it plays: the instant it started,
/// and nothing else.
///
/// A frame is a pure function of how long ago that was, so there is nothing to
/// hold in step and nothing to go stale — and the moment the piece has run out
/// the fact is taken away, leaving the resting wordmark.
#[derive(Clone, Copy, Debug)]
pub struct Playing(Instant);

impl Playing {
    pub fn from(now: Instant) -> Self {
        Playing(now)
    }

    /// Which second of the piece this frame is for.
    pub fn seconds(&self, now: Now) -> f32 {
        now.since(self.0).as_secs_f32()
    }

    /// Whether the piece has played out.
    pub fn over(&self, now: Now) -> bool {
        self.seconds(now) >= END
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::{later, scene};

    #[test]
    fn the_piece_is_over_when_its_seconds_have_run_out() {
        let (_, now) = scene();
        let playing = Playing::from(now.instant);
        assert_eq!(playing.seconds(now), 0.0);
        assert!(!playing.over(now));
        assert!(!playing.over(later(now, 2_300)));
        assert!(playing.over(later(now, 2_400)));
    }
}
