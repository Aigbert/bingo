//! Normalize legacy rows before deserializing the one live pane size.
use super::Slot;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use serde_json::Value;

impl Serialize for Slot {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        Self::serialize(self, serializer)
    }
}
impl<'de> Deserialize<'de> for Slot {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let mut value = Value::deserialize(deserializer)?;
        if value["kind"] == "pane"
            && value.get("size").is_none()
            && let Some(rows) = value.get("rows").cloned()
        {
            value["size"] = serde_json::json!({"rows":{
                "min":2,"ideal":rows,"max":"100%"}});
        }
        Self::deserialize(value).map_err(serde::de::Error::custom)
    }
}
