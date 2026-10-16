//! Payloads use the vocabulary seen by a hook, independent of its transport.
use crate::*;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeMap;

/// A denial is an answer, never an invalid output that falls through to execution.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(untagged)]
#[schemars(rename = "ToolCallAnswer")]
pub enum ToolCallResult {
    /// Refuse the call; `deny` is the reason the model reads.
    Denied { deny: String },
    /// The tool's output, from `next` or answered by the hook.
    Output(crate::ToolOutput),
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[schemars(rename = "ToolEvent")]
/// A tool call as a hook sees it: the tool's own arguments sit beside
/// `tool` and `tool_use_id`, not under an `input` field.
pub struct ToolCall {
    /// The tool's name, such as `Bash` or `Edit`.
    pub tool: String,
    /// The model's id for this call.
    pub tool_use_id: String,
    #[serde(flatten)]
    pub input: BTreeMap<String, Value>,
}

impl TryFrom<crate::ToolCall> for ToolCall {
    type Error = KernelError;
    fn try_from(call: crate::ToolCall) -> Result<Self, Self::Error> {
        let input: BTreeMap<String, Value> = serde_json::from_value(call.input)
            .map_err(|error| KernelError::new(ErrorCode::InvalidInput, error.to_string()))?;
        if let Some(field) = ["tool", "tool_use_id"]
            .into_iter()
            .find(|field| input.contains_key(*field))
        {
            return Err(KernelError::new(
                ErrorCode::InvalidInput,
                format!("tool input contains reserved field {field}"),
            ));
        }
        Ok(Self {
            tool: call.name,
            tool_use_id: call.call_id,
            input,
        })
    }
}
impl From<ToolCall> for crate::ToolCall {
    fn from(call: ToolCall) -> Self {
        Self {
            name: call.tool,
            call_id: call.tool_use_id,
            input: Value::Object(call.input.into_iter().collect()),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct Turn {
    /// The turn that ended.
    pub turn: TurnId,
    /// How it ended.
    pub outcome: TurnStatus,
    /// Tokens the turn used.
    pub usage: Usage,
    /// The assistant's final text.
    pub answer: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct Send {
    /// The receiving session.
    pub to: SessionId,
    /// What is sent.
    pub input: Input,
    /// Whether it wakes the receiver or waits for its next turn.
    pub delivery: Delivery,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct Append {
    /// The plugin that owns the record.
    pub plugin: String,
    /// The record's kind, named by its plugin.
    pub kind: String,
    /// The record itself.
    pub payload: Value,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct CommandRun {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    /// The surface the command was typed on, when known.
    pub surface: Option<crate::render::SurfaceName>,
    /// The command's name, without the slash.
    #[serde(rename = "command", alias = "name")]
    pub name: String,
    /// The rest of the line after the name.
    pub args: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(untagged)]
pub enum CommandReply {
    /// A full outcome.
    Outcome(Box<crate::CommandOutcome>),
    /// Applied, with `text` shown as its message.
    Text { text: String },
}
impl From<CommandReply> for crate::CommandOutcome {
    fn from(reply: CommandReply) -> Self {
        match reply {
            CommandReply::Outcome(outcome) => *outcome,
            CommandReply::Text { text } => Self::Applied {
                message: Some(text),
            },
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct ConfigSet {
    /// The setting's dotted key.
    pub key: String,
    /// Its new value.
    pub value: Value,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct Session {
    /// The session this concerns; `$` methods default it to the current one.
    pub session: SessionId,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct Attachment {
    /// The session.
    pub session: SessionId,
    /// The client that attached or detached.
    pub client: ClientIdentity,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct TurnStart {
    /// The turn that opened.
    pub turn: TurnId,
    /// The history items it took as input.
    pub inputs: Vec<ItemId>,
    /// What opened it.
    pub origin: TurnOrigin,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct Measure {
    /// The turn being measured.
    pub turn: TurnId,
    /// Tokens used so far.
    pub usage: Usage,
    /// The context window reading after this round.
    pub context: ContextUsage,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct Complete {
    /// The plugin that asked.
    pub plugin: String,
    /// The request it sent.
    pub request: ModelRequest,
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn arguments_cannot_replace_the_envelope_identity() {
        for field in ["tool", "tool_use_id"] {
            let mut input = serde_json::Map::new();
            input.insert(field.into(), Value::String("spoofed".into()));
            let error = ToolCall::try_from(crate::ToolCall {
                name: "Read".into(),
                call_id: "c1".into(),
                input: Value::Object(input),
            })
            .unwrap_err();
            assert_eq!(error.code, ErrorCode::InvalidInput);
            assert!(error.message.contains(field));
        }
    }
}
