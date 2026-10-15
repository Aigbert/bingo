//! One relative-time spelling for session listings.
use jiff::Timestamp;

pub fn ago(now: Timestamp, then: Timestamp) -> String {
    let seconds = (now.as_second() - then.as_second()).max(0);
    match seconds {
        0..60 => "just now".into(),
        60..3600 => format!("{}m ago", seconds / 60),
        3600..86400 => format!("{}h ago", seconds / 3600),
        86400..2592000 => format!("{}d ago", seconds / 86400),
        _ => then.to_zoned(jiff::tz::TimeZone::UTC).date().to_string(),
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn ages_and_clock_skew() {
        let now: Timestamp = "2026-10-15T12:00:00Z".parse().unwrap();
        for (age, expected) in [
            (-1, "just now"),
            (0, "just now"),
            (59, "just now"),
            (240, "4m ago"),
            (7200, "2h ago"),
            (259200, "3d ago"),
            (2592000, "2026-09-15"),
        ] {
            assert_eq!(
                ago(now, Timestamp::from_second(now.as_second() - age).unwrap()),
                expected
            );
        }
    }
}

/// A deadline in the same units as `ago`, clamped when already due.
pub fn until(now: Timestamp, then: Timestamp) -> String {
    let seconds = (then.as_second() - now.as_second()).max(0);
    match seconds {
        0 => "due now".into(),
        1..60 => format!("in {seconds}s"),
        60..3600 => format!("in {}m", seconds / 60),
        3600..86400 => format!("in {}h", seconds / 3600),
        86400..2592000 => format!("in {}d", seconds / 86400),
        _ => then.to_zoned(jiff::tz::TimeZone::UTC).date().to_string(),
    }
}
#[cfg(test)]
mod deadline_tests {
    use super::*;
    #[test]
    fn future_units_and_past_due() {
        for (secs, expected) in [
            (-1, "due now"),
            (0, "due now"),
            (40, "in 40s"),
            (240, "in 4m"),
            (7200, "in 2h"),
            (172800, "in 2d"),
        ] {
            assert_eq!(
                until(Timestamp::UNIX_EPOCH, Timestamp::from_second(secs).unwrap()),
                expected
            );
        }
    }
}
