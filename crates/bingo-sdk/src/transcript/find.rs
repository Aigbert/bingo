use crate::{ContentPart, Item, ItemBody, ItemId, Transcript, TurnId};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct Match {
    pub item: ItemId,
    pub turn: Option<TurnId>,
    pub line: usize,
}

/// One match per logical line, independent of wrapping and folding.
pub fn find(transcript: &Transcript, query: &str) -> Vec<Match> {
    if query.is_empty() {
        return Vec::new();
    }
    let query = query.to_lowercase();
    let mut hits = Vec::new();
    for item in transcript.items() {
        for (line, text) in text(item).lines().enumerate() {
            if text.to_lowercase().contains(&query) {
                hits.push(Match {
                    item: item.id.clone(),
                    turn: item.turn.clone(),
                    line,
                });
                if hits.len() == 1000 {
                    return hits;
                }
            }
        }
    }
    hits
}

/// Full searchable and copyable text; presentation never limits this projection.
pub fn text(item: &Item) -> String {
    match &item.body {
        ItemBody::User { parts, .. } => parts
            .iter()
            .filter_map(ContentPart::as_text)
            .collect::<Vec<_>>()
            .join(""),
        ItemBody::Assistant { text } | ItemBody::Reasoning { text, .. } => text.clone(),
        ItemBody::ToolCall {
            name,
            input,
            output,
            ..
        } => {
            let result = output
                .as_ref()
                .map(|o| {
                    o.parts
                        .iter()
                        .filter_map(ContentPart::as_text)
                        .collect::<Vec<_>>()
                        .join("\n")
                })
                .unwrap_or_default();
            format!("{name}\n{}\n{result}", readable(input))
        }
        ItemBody::Shell {
            command, output, ..
        } => format!("{command}\n{output}"),
        ItemBody::Compaction { summary, .. } => summary.clone(),
        _ => serde_json::to_string(&item.body).unwrap_or_default(),
    }
}

fn readable(value: &serde_json::Value) -> String {
    match value {
        serde_json::Value::String(text) => text.clone(),
        serde_json::Value::Array(values) => {
            values.iter().map(readable).collect::<Vec<_>>().join("\n")
        }
        serde_json::Value::Object(fields) => fields
            .iter()
            .map(|(name, value)| format!("{name}:\n{}", readable(value)))
            .collect::<Vec<_>>()
            .join("\n"),
        value => value.to_string(),
    }
}
