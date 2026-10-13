//! Two remote phases around a host continuation.
use std::collections::BTreeMap;
use bingo_sdk::*;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use crate::wire::HookSite;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct LinkSpec {
    pub point: LinkPoint,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tool: Option<String>,
    #[serde(default)]
    pub after: bool,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct Before<I> {
    pub chain: String,
    pub tier: Tier,
    pub site: HookSite,
    pub input: I,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub enum Proceed<I, O> {
    Next(I),
    Answer(O),
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct After<O> {
    pub chain: String,
    pub result: O,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct ResultValue<O> { pub result: O }
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct Cancel { pub chain: String }

/// Catalogue spelling: tool input fields live beside the pinned identity.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct ToolEvent {
    pub tool: String,
    pub tool_use_id: String,
    #[serde(flatten)]
    pub input: BTreeMap<String, Value>,
}


#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn catalogue_tool_fields_and_completion_spelling() {
        let event: ToolEvent = serde_json::from_value(serde_json::json!({"tool":"Bash","tool_use_id":"c1","command":"ls"})).unwrap();
        assert_eq!(event.input["command"], "ls");
        assert_eq!(serde_json::to_value(LinkPoint::TurnStop).unwrap(), "turn.complete");
        let next = Proceed::<_, ToolOutput>::Next(event);
        let value = serde_json::to_value(next).unwrap();
        assert_eq!(value["next"]["tool_use_id"], "c1");
        assert!(value["next"].get("input").is_none());
        let result = serde_json::to_value(ResultValue {result: ToolOutput::error("failed")}).unwrap();
        assert_eq!(result["result"]["isError"], true);
    }
}
