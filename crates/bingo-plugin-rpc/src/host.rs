//! Reverse requests translate wire envelopes onto the host's own doors.

use bingo_sdk::{Input, ModelEvent, ModelRequest, SessionId, TurnId};
use bingo_sdk::slot::Slot;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
pub struct Place {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub session: Option<SessionId>,
    /// The plugin's placement key. Several independent slots can coexist.
    pub slot: String,
    pub view: Option<Slot>,
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
pub struct Extend {
    pub session: SessionId,
    pub key: String,
    pub value: Value,
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
pub struct Deliver {
    pub session: SessionId,
    pub input: Input,
    pub wake: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct StateGet {
    pub scope: String,
    pub key: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct StateSet {
    pub scope: String,
    pub key: String,
    pub value: Value,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub if_version: Option<u64>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct StateValue {
    pub version: u64,
    pub value: Value,
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
pub struct Complete {
    pub session: SessionId,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub turn: Option<TurnId>,
    pub request: ModelRequest,
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
pub struct Completed {
    pub events: Vec<ModelEvent>,
}
