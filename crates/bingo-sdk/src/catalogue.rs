//! The named interception contract, shared by every transport.
use schemars::{JsonSchema, Schema};
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub enum Kind {
    Chain,
    Observation,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct Acts {
    pub deny: bool,
    pub rewrite: bool,
    pub answer: bool,
}

/// Only the catalogue constructs kernel entries.
pub struct Entry {
    pub name: &'static str,
    pub kind: Kind,
    pub input: fn() -> Schema,
    pub result: fn() -> Schema,
    pub matcher: &'static [&'static str],
    pub pinned: &'static [&'static str],
    pub acts: Acts,
}

pub fn document() -> Value {
    Value::Null
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_catalogue_names_both_turn_edges() {
        let document = document();
        let events = &document["events"];
        assert!(events.get("turn.stop").is_some(), "the stop chain is named");
        assert!(events.get("turn.complete").is_some(), "completion is distinct");
    }

    #[test]
    fn the_committed_catalogue_is_generated() {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../schema/events.json");
        let document = document();
        assert!(document["events"].is_object(), "catalogue must contain entries");
        let rendered = format!("{}\n", serde_json::to_string_pretty(&document).unwrap());
        if std::env::var_os("BINGO_UPDATE_SCHEMA").is_some() {
            std::fs::write(path, rendered).unwrap();
        } else {
            assert_eq!(std::fs::read_to_string(path).unwrap_or_default(), rendered,
                "regenerate with BINGO_UPDATE_SCHEMA=1 cargo test -p bingo-sdk catalogue::");
        }
    }
}
