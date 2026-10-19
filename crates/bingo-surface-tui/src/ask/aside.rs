//! An ask that opens while the person is typing leaves the keys with the
//! composer, so words meant for it do not land in the ask.
use jiff::Timestamp;

pub const RECENT_MS: i64 = 900;

pub fn recent(received: Timestamp, typed: Option<Timestamp>) -> bool {
    typed.is_some_and(|typed| {
        let elapsed = received.as_millisecond() - typed.as_millisecond();
        (0..RECENT_MS).contains(&elapsed)
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn recent_typing_uses_an_exact_window_without_wall_time() {
        let opened = Timestamp::from_millisecond(10_000).unwrap();
        for (delta, expected) in [
            (0, true),
            (899, true),
            (900, false),
            (901, false),
            (-1, false),
        ] {
            assert_eq!(
                recent(
                    opened,
                    Some(Timestamp::from_millisecond(10_000 - delta).unwrap())
                ),
                expected
            );
        }
        assert!(!recent(opened, None));
    }
}
