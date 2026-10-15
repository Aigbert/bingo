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
    transcript
        .items()
        .iter()
        .flat_map(|item| {
            text(item)
                .lines()
                .enumerate()
                .filter(|(_, line)| line.to_lowercase().contains(&query))
                .map(|(line, _)| Match {
                    item: item.id.clone(),
                    turn: item.turn.clone(),
                    line,
                })
                .collect::<Vec<_>>()
        })
        .collect()
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
            format!("{name} {input}\n{result}")
        }
        ItemBody::Shell {
            command, output, ..
        } => format!("{command}\n{output}"),
        ItemBody::Compaction { summary, .. } => summary.clone(),
        _ => serde_json::to_string(&item.body).unwrap_or_default(),
    }
}
