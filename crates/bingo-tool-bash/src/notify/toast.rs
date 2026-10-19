//! A completion toast, independent of the running jobs signal.
use super::tone;
use crate::jobs::State;
use std::time::Duration;

/// A completion has its own signal so it cannot replace the running jobs.
pub const TOAST: &str = "job-finished";

pub fn toast(now: jiff::Timestamp, state: State, outcome: &str) -> serde_json::Value {
    bingo_sdk::slot::with(
        serde_json::json!({}),
        [bingo_sdk::slot::Slot::Toast {
            text: format!("background job finished · {outcome}"),
            tone: tone(state),
            until: now + Duration::from_millis(2400),
            alert: false,
        }],
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn completion_toast_has_its_own_kind_tone_and_fixed_deadline() {
        let now: jiff::Timestamp = "2026-10-14T12:00:00Z".parse().unwrap();
        assert_ne!(TOAST, crate::jobs::KIND);
        for (state, tone) in [
            (State::Exited { code: 0 }, "good"),
            (State::Exited { code: 2 }, "bad"),
            (State::Killed, "neutral"),
        ] {
            let payload = toast(now, state, "2 tests failed");
            let slot = &payload["slots"][0];
            assert_eq!(slot["text"], "background job finished · 2 tests failed");
            assert_eq!(slot["until"], "2026-10-14T12:00:02.4Z");
            assert_eq!(slot["alert"], false);
            assert_eq!(
                slot.get("tone")
                    .and_then(|v| v.as_str())
                    .unwrap_or("neutral"),
                tone
            );
        }
    }
}
