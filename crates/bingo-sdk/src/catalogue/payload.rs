//! Payloads use the vocabulary seen by a hook, independent of its transport.
use crate::*;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeMap;

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
        let input = serde_json::from_value(call.input)
            .map_err(|error| KernelError::new(ErrorCode::InvalidInput, error.to_string()))?;
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
    pub name: String,
    pub args: String,
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
