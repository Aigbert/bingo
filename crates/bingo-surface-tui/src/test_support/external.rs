use bingo_sdk::{Item, ItemBody, ItemStatus, ToolOutput};
use serde_json::Value;

use super::{item, ts};

/// Build an external call fixture from its title, kind and reported text.
pub fn agent_call(id: &str, text: &str, fields: Value) -> Item {
    if fields.get("external") != Some(&Value::Bool(true)) {
        let mut thought = item(
            id,
            ItemStatus::Completed,
            ItemBody::Reasoning {
                text: text.into(),
                provider_metadata: Default::default(),
            },
        );
        thought.completed_at = Some(ts() + jiff::SignedDuration::from_secs(1));
        return thought;
    }
    let kind = serde_json::from_value(fields["kind"].clone()).unwrap_or(bingo_sdk::CallKind::Other);
    let external = bingo_sdk::ExternalCall {
        id: id.into(),
        kind,
        title: fields["title"].as_str().unwrap_or("Tool").into(),
        locations: vec![],
        raw: fields["rawInput"].clone(),
    };
    let text = fields["content"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|c| c.pointer("/content/text").and_then(Value::as_str))
        .collect::<Vec<_>>()
        .join("\n");
    let failed = fields["status"] == "failed";
    let call = external.tool_call("external");
    let mut result = item(
        id,
        if failed {
            ItemStatus::Failed
        } else {
            ItemStatus::Completed
        },
        ItemBody::ToolCall {
            allowed_by: None,
            call_id: call.call_id,
            name: call.name,
            input: call.input,
            output: Some(if failed {
                ToolOutput::error(text)
            } else {
                ToolOutput::text(text)
            }),
            progress: None,
            duration_ms: None,
            quiet: false,
        },
    );
    result.mark_external();
    result.completed_at = Some(ts() + jiff::SignedDuration::from_secs(1));
    result
}
