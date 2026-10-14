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
#[derive(Clone, Debug)]
pub struct Entry {
    pub name: std::borrow::Cow<'static, str>,
    pub kind: Kind,
    pub input: Shape,
    pub result: Shape,
    pub matcher: &'static [&'static str],
    pub pinned: &'static [&'static str],
    pub acts: Acts,
    pub valid_input: fn(&Value) -> bool,
    pub valid_result: fn(&Value) -> bool,
}

/// Native entries derive schemas from Rust; external faces supply declared schemas.
#[derive(Clone, Debug)]
pub enum Shape {
    Rust(fn() -> Schema),
    Declared(Schema),
}
impl Shape {
    pub fn schema(&self) -> Schema {
        match self {
            Self::Rust(schema) => schema(),
            Self::Declared(schema) => schema.clone(),
        }
    }
}
impl Entry {
    pub fn face(name: String, input: Schema, result: Schema) -> Self {
        Self {
            name: name.into(),
            kind: Kind::Chain,
            input: Shape::Declared(input),
            result: Shape::Declared(result),
            matcher: &[],
            pinned: &[],
            acts: WRAP,
            valid_input: |_| true,
            valid_result: |_| true,
        }
    }
}

mod matcher;
pub use matcher::{Pattern, pinned_changed, prefilter};
mod payload;
pub use payload::*;

impl PartialEq for Entry {
    fn eq(&self, other: &Self) -> bool {
        self.name == other.name
    }
}
impl Eq for Entry {}
impl Serialize for Entry {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&self.name)
    }
}
impl<'de> Deserialize<'de> for &'static Entry {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let name = String::deserialize(deserializer)?;
        find(&name).ok_or_else(|| serde::de::Error::custom(format!("unknown event: {name}")))
    }
}
impl JsonSchema for Entry {
    fn schema_name() -> std::borrow::Cow<'static, str> {
        "LinkPoint".into()
    }
    fn json_schema(_: &mut schemars::SchemaGenerator) -> Schema {
        schemars::json_schema!({"type":"string", "enum": ALL.iter().map(|e| e.name.as_ref()).collect::<Vec<_>>()})
    }
}

macro_rules! catalogue {
    ($($id:ident: $name:literal, $kind:ident, $input:ty => $result:ty,
       $matcher:expr, $pinned:expr, $acts:expr;)+) => {
        $(pub const $id: &Entry = &Entry {
            name: std::borrow::Cow::Borrowed($name), kind: Kind::$kind,
            input: Shape::Rust(|| schemars::schema_for!($input)), result: Shape::Rust(|| schemars::schema_for!($result)),
            matcher: $matcher, pinned: $pinned, acts: $acts,
            valid_input: |value| serde_json::from_value::<$input>(value.clone()).is_ok(),
            valid_result: |value| serde_json::from_value::<$result>(value.clone()).is_ok(),
        };)+
        pub const ALL: &[&Entry] = &[$($id),+];
    };
}
const WRAP: Acts = Acts {
    deny: true,
    rewrite: true,
    answer: true,
};
const PASS: Acts = Acts {
    deny: true,
    rewrite: true,
    answer: false,
};
const WATCH: Acts = Acts {
    deny: false,
    rewrite: false,
    answer: false,
};

catalogue! {
    UI_PRESS: "ui.press", Chain, crate::render::HandlerEvent => (), &["handler", "event"], &["/handler", "/event"], WRAP;
    UI_INPUT: "ui.input", Chain, crate::render::HandlerEvent => (), &["handler", "event"], &["/handler", "/event"], WRAP;
    UI_SELECT: "ui.select", Chain, crate::render::HandlerEvent => (), &["handler", "event"], &["/handler", "/event"], WRAP;
    UI_RENDER: "ui.render", Chain, crate::render::RenderRequest => crate::View, &["component", "requestId", "surface"], &["/component", "/requestId", "/surface", "/viewport"], WRAP;

    PLUGIN_REGISTER: "plugin.register", Chain, crate::PluginRegistration => crate::PluginDecision, &["name", "origin", "runtime"], &["/name", "/version", "/origin", "/runtime"], WRAP;
    PROMPT_SUBMIT: "prompt.submit", Chain, crate::Input => crate::SubmitResult, &[], &[], PASS;
    TOOL_CHECK: "tool.check", Chain, ToolCall => crate::CheckResult, &["tool"], &["/tool", "/tool_use_id"], PASS;
    TOOL_CALL: "tool.call", Chain, ToolCall => ToolCallResult, &["tool"], &["/tool", "/tool_use_id"], WRAP;
    MODEL_COMPLETE: "model.complete", Chain, Complete => Vec<crate::ModelEvent>, &["plugin"], &["/plugin"], WRAP;
    TURN_STEP: "turn.step", Chain, crate::ModelRequest => Result<Vec<crate::ModelEvent>, crate::ProviderError>, &[], &[], WRAP;
    PROMPT_COMPOSE: "prompt.compose", Chain, crate::ModelRequest => crate::ModelRequest, &[], &[], WRAP;
    SESSION_COMPACT: "session.compact", Chain, crate::CompactInput => Result<crate::Compaction, crate::CompactError>, &[], &[], WRAP;
    TURN_STOP: "turn.stop", Chain, () => crate::StopResult, &[], &[], PASS;
    TOOL_DESCRIBE: "tool.describe", Chain, crate::ToolSpec => Option<crate::ToolSpec>, &["name"], &["/name"], WRAP;
    PROMPT_AUTOCOMPLETE: "prompt.autocomplete", Chain, crate::Catalog => crate::Catalog, &["kind"], &["/kind"], WRAP;
    SESSION_SEND: "session.send", Chain, Send => (), &["to"], &[], PASS;
    SESSION_RECEIVE: "session.receive", Chain, crate::Input => crate::Input, &["origin.surface"], &[], PASS;
    SESSION_APPEND: "session.append", Chain, Append => Append, &["plugin", "kind"], &["/plugin", "/kind"], PASS;
    TURN_COMPLETE: "turn.complete", Chain, Turn => Option<String>, &[], &["/turn", "/outcome", "/usage", "/answer"], WRAP;
    COMMAND_RUN: "command.run", Chain, CommandRun => CommandReply, &["command"], &["/command"], WRAP;
    COMMAND_DESCRIBE: "command.describe", Chain, crate::CommandSpec => Option<crate::CommandSpec>, &["name"], &["/name"], WRAP;
    CONFIG_SET: "config.set", Chain, ConfigSet => ConfigSet, &["key"], &["/key"], PASS;
    CONFIG_DESCRIBE: "config.describe", Chain, crate::ConfigView => crate::ConfigView, &[], &[], WRAP;
    TURN_START: "turn.start", Observation, TurnStart => (), &[], &[], WATCH;
    SESSION_MEASURE: "session.measure", Observation, Measure => (), &[], &[], WATCH;
    SESSION_START: "session.start", Observation, Session => (), &[], &[], WATCH;
    SESSION_END: "session.end", Observation, Session => (), &[], &[], WATCH;
    SESSION_ATTACH: "session.attach", Observation, Attachment => (), &["client.surface"], &[], WATCH;
    SESSION_DETACH: "session.detach", Observation, Attachment => (), &["client.surface"], &[], WATCH;
}

/// Reserved envelope fields cannot also be tool arguments.
pub fn reserved_tool_field(schema: &Value) -> Option<&'static str> {
    ["tool", "tool_use_id"].into_iter().find(|field| {
        schema
            .get("properties")
            .and_then(|fields| fields.get(*field))
            .is_some()
    })
}

pub fn find(name: &str) -> Option<&'static Entry> {
    ALL.iter().copied().find(|entry| entry.name == name)
}

pub fn document() -> Value {
    let events: serde_json::Map<String, Value> = ALL
        .iter()
        .map(|entry| {
            (
                entry.name.to_string(),
                serde_json::json!({
                    "kind":entry.kind, "input":entry.input.schema(), "result":entry.result.schema(),
                    "matcher":entry.matcher, "pinned":entry.pinned, "acts":entry.acts,
                }),
            )
        })
        .collect();
    serde_json::json!({"events":events})
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_catalogue_names_both_turn_edges() {
        let document = document();
        let events = &document["events"];
        assert!(events.get("turn.stop").is_some(), "the stop chain is named");
        assert!(
            events.get("turn.complete").is_some(),
            "completion is distinct"
        );
    }

    #[test]
    fn command_hooks_use_the_command_field_and_text_answers() {
        let input = CommandRun {
            name: "inspect".into(),
            args: "".into(),
        };
        assert_eq!(
            serde_json::to_value(input).unwrap(),
            serde_json::json!({"command":"inspect","args":""})
        );
        assert!((COMMAND_RUN.valid_result)(
            &serde_json::json!({"text":"opened"})
        ));
        assert_eq!(COMMAND_RUN.matcher, &["command"]);
        assert_eq!(COMMAND_RUN.pinned, &["/command"]);
    }

    #[test]
    fn the_committed_catalogue_is_generated() {
        let path =
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../schema/events.json");
        let document = document();
        assert!(
            document["events"].is_object(),
            "catalogue must contain entries"
        );
        let rendered = format!("{}\n", serde_json::to_string_pretty(&document).unwrap());
        if std::env::var_os("BINGO_UPDATE_SCHEMA").is_some() {
            std::fs::write(path, rendered).unwrap();
        } else {
            assert_eq!(
                std::fs::read_to_string(path).unwrap_or_default(),
                rendered,
                "regenerate with BINGO_UPDATE_SCHEMA=1 cargo test -p bingo-sdk catalogue::"
            );
        }
    }
}
