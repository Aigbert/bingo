//! Read answers are projections of the client's one fold.
use bingo_sdk::{ItemBody, SessionState};
use serde_json::{Value, json};

pub fn read(state: &SessionState, method: &str, key: Option<&str>) -> Result<Value, String> {
    Ok(match method {
        "session.id" => json!(state.summary.id),
        "session.cwd" => json!(state.summary.cwd),
        "session.model" => json!(state.summary.model),
        "session.turns" => turns(state),
        "session.usage" => json!(super::params::UsageView {
            session: state.summary.usage,
            context: state.context,
            last_turn: state.last_turn.as_ref().map(|turn| turn.usage),
        }),
        "session.surfaces" => surfaces(state),
        "config.list" => json!(state.config),
        "config.read" => {
            let key = key.ok_or("config.read requires a key")?;
            state
                .config
                .kernel
                .get(key)
                .or_else(|| state.config.plugins.get(key))
                .cloned()
                .unwrap_or(Value::Null)
        }
        "prompt.read" => json!(super::params::PromptView {
            queue: state.queue.clone(),
            last: last_prompt(state)
        }),
        _ => return Err(format!("unknown folded read: {method}")),
    })
}

fn turns(state: &SessionState) -> Value {
    let mut seen = std::collections::BTreeSet::new();
    let turns: Vec<_> = state
        .transcript
        .items()
        .iter()
        .filter_map(|item| item.turn.as_ref())
        .chain(state.last_turn.iter().map(|turn| &turn.id))
        .chain(state.turn.iter().map(|turn| &turn.id))
        .filter(|turn| seen.insert(*turn))
        .collect();
    json!(turns)
}
fn last_prompt(state: &SessionState) -> Option<String> {
    state
        .transcript
        .items()
        .iter()
        .rev()
        .find_map(|item| match &item.body {
            ItemBody::User { parts, origin } if !origin.is_context() => Some(
                parts
                    .iter()
                    .filter_map(|part| part.as_text())
                    .collect::<Vec<_>>()
                    .join("\n"),
            ),
            _ => None,
        })
}
fn surfaces(state: &SessionState) -> Value {
    json!(
        state
            .signals
            .get("bingo.mods")
            .into_iter()
            .flat_map(|signals| signals.iter())
            .filter(|(key, value)| key.starts_with("surface/")
                && value["count"].as_u64().unwrap_or(0) > 0)
            .map(|(_, value)| &value["client"])
            .collect::<Vec<_>>()
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use bingo_sdk::*;
    use serde_json::json;

    #[test]
    fn answers_are_fields_of_the_same_fold() {
        let mut state = SessionState::new(
            serde_json::from_value(json!({
                "id":"s", "cwd":"/work", "model":"m", "createdAt":"2026-01-01T00:00:00Z",
                "updatedAt":"2026-01-01T00:00:00Z", "usage":Usage::default()
            }))
            .unwrap(),
        );
        state.apply(&serde_json::from_value(json!({"seq":1, "ts":"2026-01-01T00:00:01Z", "session":"s",
            "event":{"type":"configChanged", "config":{"kernel":{"thinking":"high"},"plugins":{"example":{"flag":true}}}}
        })).unwrap());
        for (method, key, expected) in [
            ("session.id", None, json!(state.summary.id)),
            ("session.cwd", None, json!(state.summary.cwd)),
            ("session.model", None, json!(state.summary.model)),
            ("session.turns", None, json!([])),
            (
                "session.usage",
                None,
                json!({"session":state.summary.usage,"context":state.context,"lastTurn":null}),
            ),
            ("config.list", None, json!(state.config)),
            ("config.read", Some("thinking"), json!("high")),
            ("config.read", Some("example"), json!({"flag":true})),
            (
                "prompt.read",
                None,
                json!({"queue":state.queue,"last":null}),
            ),
            ("session.surfaces", None, json!([])),
        ] {
            assert_eq!(read(&state, method, key).unwrap(), expected, "{method}");
        }
    }
}
