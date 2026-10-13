//! Read answers are projections of the client's one fold.
use bingo_sdk::SessionState;
use serde_json::Value;

pub fn read(_state: &SessionState, method: &str, _key: Option<&str>) -> Result<Value, String> {
    Err(format!("unknown folded read: {method}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use bingo_sdk::*;
    use serde_json::json;

    #[test]
    fn answers_are_fields_of_the_same_fold() {
        let mut state = SessionState::new(serde_json::from_value(json!({
            "id":"s", "cwd":"/work", "model":"m", "createdAt":"2026-01-01T00:00:00Z",
            "updatedAt":"2026-01-01T00:00:00Z", "usage":Usage::default()
        })).unwrap());
        state.apply(&serde_json::from_value(json!({"seq":1, "ts":"2026-01-01T00:00:01Z", "session":"s",
            "event":{"type":"configChanged", "config":{"kernel":{"thinking":"high"},"plugins":{"example":{"flag":true}}}}
        })).unwrap());
        for (method, key, expected) in [
            ("session.id", None, json!(state.summary.id)),
            ("session.cwd", None, json!(state.summary.cwd)),
            ("session.model", None, json!(state.summary.model)),
            ("session.turns", None, json!([])),
            ("session.usage", None, json!({"session":state.summary.usage,"context":state.context,"lastTurn":null})),
            ("config.list", None, json!(state.config)),
            ("config.read", Some("thinking"), json!("high")),
            ("config.read", Some("example"), json!({"flag":true})),
            ("prompt.read", None, json!({"queue":state.queue,"last":null})),
            ("session.surfaces", None, json!([])),
        ] {
            assert_eq!(read(&state, method, key).unwrap(), expected, "{method}");
        }
    }
}
