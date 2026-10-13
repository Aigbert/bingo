//! Each step upgrades one journal version before the live frame is decoded.

use serde_json::Value;

pub(super) const STEPS: &[fn(Value) -> Value] = &[std::convert::identity, external_call];

pub(super) fn upgrade(value: Value, version: u32, steps: &[fn(Value) -> Value]) -> Value {
    steps
        .iter()
        .skip(version.saturating_sub(1) as usize)
        .fold(value, |value, step| step(value))
}

/// Identity steps must not normalize the bytes of opaque event/item payloads.
pub(super) fn line(
    bytes: &[u8],
    version: u32,
) -> Result<std::borrow::Cow<'_, [u8]>, serde_json::Error> {
    if version == super::VERSION {
        return Ok(std::borrow::Cow::Borrowed(bytes));
    }
    let value: Value = serde_json::from_slice(bytes)?;
    let migrated = upgrade(value.clone(), version, STEPS);
    if migrated == value {
        Ok(std::borrow::Cow::Borrowed(bytes))
    } else {
        serde_json::to_vec(&migrated).map(std::borrow::Cow::Owned)
    }
}

fn external_call(mut frame: Value) -> Value {
    let Some(item) = frame.pointer_mut("/event/item") else {
        return frame;
    };
    let Some(body) = item.get("body") else {
        return frame;
    };
    if body.get("kind").and_then(Value::as_str) != Some("reasoning") {
        return frame;
    }
    let Some(old) = body.pointer("/providerMetadata/acp") else {
        return frame;
    };
    if old.get("external").and_then(Value::as_bool) != Some(true) {
        return frame;
    }
    let Some(id) = old.get("toolCallId").and_then(Value::as_str) else {
        return frame;
    };
    let kind = old.get("kind").and_then(Value::as_str).unwrap_or("other");
    let kind = match kind {
        "read" | "edit" | "delete" | "move" | "search" | "execute" | "think" | "fetch" => kind,
        _ => "other",
    };
    let mut text = old
        .get("content")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|c| c.pointer("/content/text").and_then(Value::as_str))
        .collect::<Vec<_>>()
        .join("\n");
    if text.is_empty() {
        text = body
            .get("text")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_owned();
    }
    let output = serde_json::json!({"parts":[{"type":"text","text":text}],"isError":old.get("status").and_then(Value::as_str) == Some("failed")});
    let body = serde_json::json!({"kind":"toolCall","callId":id,"name":format!("acp:{kind}"),
        "input":{"title":old.get("title").and_then(Value::as_str).unwrap_or(id),
            "locations":old.get("locations").cloned().unwrap_or_else(|| serde_json::json!([])),
            "raw":old.get("rawInput").cloned().unwrap_or(Value::Null)},
        "output":output});
    item["body"] = body;
    if !item.get("meta").is_some_and(Value::is_object) {
        item["meta"] = serde_json::json!({});
    }
    item["meta"]["external"] = Value::Bool(true);
    frame
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn every_version_has_exactly_one_step_to_its_successor() {
        assert_eq!(STEPS.len(), super::super::VERSION as usize - 1);
    }

    #[test]
    fn steps_chain_from_the_lines_version_before_decoding() {
        fn rename(mut value: Value) -> Value {
            value["new"] = value.as_object_mut().unwrap().remove("old").unwrap();
            value
        }
        fn wrap(value: Value) -> Value {
            json!({"wrapped": value})
        }
        let steps: &[fn(Value) -> Value] = &[rename, wrap];
        assert_eq!(
            upgrade(json!({"old": 42}), 1, steps),
            json!({"wrapped": {"new": 42}})
        );
        assert_eq!(
            upgrade(json!({"new": 42}), 2, steps),
            json!({"wrapped": {"new": 42}})
        );
        let current = json!({"wrapped": {"new": 42}});
        assert_eq!(upgrade(current.clone(), 3, steps), current);
    }
}
