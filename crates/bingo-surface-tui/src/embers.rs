//! Token crossings leave brief light in the activity band's reserved air.
use std::time::{Duration, Instant};

pub const LIFE: Duration = Duration::from_millis(600);

pub fn crossings(_before: u64, _after: u64, _now: Instant) -> Vec<Instant> {
    Vec::new()
}

pub fn alive(births: &[Instant], now: Instant, motion: bool) -> Vec<Instant> {
    if !motion { return Vec::new(); }
    births.iter().copied().filter(|born| now.saturating_duration_since(*born) < LIFE).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn tokens_cross_in_sixty_fours_and_leave_at_most_four_embers() {
        let now = Instant::now();
        assert!(crossings(0, 63, now).is_empty());
        assert_eq!(crossings(63, 64, now), vec![now]);
        assert_eq!(crossings(64, 1024, now), vec![now; 4]);
        assert!(crossings(128, 64, now).is_empty());
        assert!(alive(&[now], now + LIFE, true).is_empty());
        assert!(alive(&[now], now, false).is_empty());
    }
}
