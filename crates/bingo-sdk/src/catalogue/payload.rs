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
    Denied { deny: String },
    Output(crate::ToolOutput),
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[schemars(rename = "ToolEvent")]
pub struct ToolCall {
    pub tool: String,
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
    pub turn: TurnId,
    pub outcome: TurnStatus,
    pub usage: Usage,
    pub answer: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct Send {
    pub to: SessionId,
    pub input: Input,
    pub delivery: Delivery,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct Append {
    pub plugin: String,
    pub kind: String,
    pub payload: Value,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct CommandRun {
    #[serde(rename = "command", alias = "name")]
    pub name: String,
    pub args: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(untagged)]
pub enum CommandReply {
    Outcome(Box<crate::CommandOutcome>),
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
    pub key: String,
    pub value: Value,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct Session {
    pub session: SessionId,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct Attachment {
    pub session: SessionId,
    pub client: ClientIdentity,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct TurnStart {
    pub turn: TurnId,
    pub inputs: Vec<ItemId>,
    pub origin: TurnOrigin,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct Measure {
    pub turn: TurnId,
    pub usage: Usage,
    pub context: ContextUsage,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct Complete {
    pub plugin: String,
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
