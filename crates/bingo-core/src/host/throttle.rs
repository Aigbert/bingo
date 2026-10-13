//! A rolling second bounds invalidation frames per request.
use std::collections::{BTreeMap, VecDeque};
use std::time::Duration;

#[derive(Default)]
pub(super) struct Throttle(BTreeMap<String, VecDeque<Duration>>);
impl Throttle {
    pub(super) fn admit(&mut self, request: &str, now: Duration) -> bool {
        let times = self.0.entry(request.into()).or_default();
        times.push_back(now);
        true
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn a_request_sends_at_most_thirty_frames_in_a_rolling_second() {
        let mut throttle = Throttle::default();
        let admitted = (0..100).filter(|_| throttle.admit("counter", Duration::ZERO)).count();
        assert_eq!(admitted, 30);
        assert!(!throttle.admit("counter", Duration::from_millis(999)));
        assert!(throttle.admit("other", Duration::from_millis(999)));
        assert!(throttle.admit("counter", Duration::from_secs(1)));
    }
}
