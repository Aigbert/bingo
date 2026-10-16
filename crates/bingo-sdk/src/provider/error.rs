//! Parse response error details without performing I/O.
use serde_json::Value;

pub fn message_of(body: &str) -> String {
    let Ok(value) = serde_json::from_str::<Value>(body) else {
        return body.trim().to_string();
    };
    value
        .pointer("/error/message")
        .or_else(|| value.pointer("/message"))
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|m| !m.is_empty())
        .map(str::to_string)
        .unwrap_or_else(|| body.trim().to_string())
}

pub fn retry_after_ms(value: &str, now: jiff::Timestamp) -> Option<u64> {
    let value = value.trim();
    if let Ok(seconds) = value.parse::<u64>() {
        return Some(seconds.saturating_mul(1_000));
    }
    let until = jiff::fmt::rfc2822::parse(value).ok()?.timestamp();
    let millis = until.duration_since(now).as_millis();
    Some(u64::try_from(millis).unwrap_or(0))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn error_messages_keep_bare_bodies_and_unwrap_envelopes() {
        for (body, expected) in [
            (
                r#"{"error":{"code":"invalid_value","message":"boom"}}"#,
                "boom",
            ),
            (r#"{"error":{"type":"api_error","message":"boom"}}"#, "boom"),
            (r#"{"message":"plain"}"#, "plain"),
            ("  Bad Gateway  ", "Bad Gateway"),
            ("  gateway timeout  ", "gateway timeout"),
            ("{}", "{}"),
        ] {
            assert_eq!(message_of(body), expected);
        }
    }

    #[test]
    fn retry_dates_are_relative_to_the_supplied_clock() {
        let now: jiff::Timestamp = "2026-10-16T00:00:00Z".parse().unwrap();
        for (value, expected) in [
            ("Fri, 16 Oct 2026 00:02:00 GMT", Some(120_000)),
            ("Sun, 06 Nov 1994 08:49:37 GMT", Some(0)),
            ("not a date", None),
            (" 7 ", Some(7_000)),
            ("18446744073709551615", Some(u64::MAX)),
        ] {
            assert_eq!(retry_after_ms(value, now), expected, "{value}");
        }
    }
}
