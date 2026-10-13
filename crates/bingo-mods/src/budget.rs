//! Active time, independent of the clock that supplies it.
use std::time::Duration;

#[derive(Clone, Debug, Default)]
pub struct Budget {
    resumed: Option<Duration>,
    spent: Duration,
}

impl Budget {
    pub fn resume(&mut self, now: Duration) {
        self.resumed.get_or_insert(now);
    }
    pub fn suspend(&mut self, now: Duration) {
        if let Some(start) = self.resumed.take() {
            self.spent += now.saturating_sub(start);
        }
    }
    pub fn spent(&self, now: Duration) -> Duration {
        self.spent
            + self
                .resumed
                .map_or(Duration::ZERO, |start| now.saturating_sub(start))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn only_resumed_intervals_spend_the_budget() {
        let mut budget = Budget::default();
        for (resume, at, expected) in [
            (true, 10, 0),
            (true, 20, 10),
            (false, 30, 20),
            (false, 500, 20),
            (true, 600, 20),
            (false, 615, 35),
        ] {
            let now = Duration::from_millis(at);
            if resume {
                budget.resume(now);
            } else {
                budget.suspend(now);
            }
            assert_eq!(budget.spent(now), Duration::from_millis(expected));
        }
    }
}
