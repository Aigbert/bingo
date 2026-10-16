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
    /// The doc comment lines of the catalogue row; empty for a service face.
    pub doc: &'static [&'static str],
    pub kind: Kind,
    pub shapes_request: bool,
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
            doc: &[],
            kind: Kind::Chain,
            shapes_request: false,
            input: Shape::Declared(input),
            result: Shape::Declared(result),
            matcher: &[],
            pinned: &[],
            acts: WRAP,
            valid_input: |_| true,
            valid_result: |_| true,
        }
    }

    /// What the event is and what `next` does, from the row's doc comment.
    pub fn description(&self) -> Option<String> {
        let lines: Vec<&str> = self.doc.iter().map(|line| line.trim()).collect();
        (!lines.is_empty()).then(|| lines.join(" "))
    }

    /// The same point holding `pinned` in place of the catalogue's pins; a
    /// dispatch that must hold more of its input passes a superset.
    pub fn pinning(&self, pinned: &'static [&'static str]) -> Self {
        Self {
            pinned,
            ..self.clone()
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
    ($($(#[doc = $doc:literal])+ $id:ident: $name:literal, $kind:ident, $input:ty => $result:ty,
        $matcher:expr, $pinned:expr, $acts:expr $(, $shapes:literal)?;)+) => {
        $(pub const $id: &Entry = &Entry {
            name: std::borrow::Cow::Borrowed($name), kind: Kind::$kind, doc: &[$($doc),+],
            input: Shape::Rust(|| schemars::schema_for!($input)), result: Shape::Rust(|| schemars::schema_for!($result)),
            matcher: $matcher, pinned: $pinned, acts: $acts,
            shapes_request: false $(|| $shapes)?,
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
    /// A press on a control a mod drew: `handler` is the token its `onPress`
    /// or `onKey` became, `value` what the surface sent. `next` runs that
    /// handler; return `{handled: false}` to give a key back to the surface.
    UI_PRESS: "ui.press", Chain, crate::render::HandlerEvent => Option<crate::render::HandlerResult>, &["handler", "event"], &["/handler", "/event"], WRAP;
    /// Text entered in an `Input` a mod drew (`onInput`, `onSubmit`): `value`
    /// is the text. `next` runs the handler.
    UI_INPUT: "ui.input", Chain, crate::render::HandlerEvent => (), &["handler", "event"], &["/handler", "/event"], WRAP;
    /// A choice made in a `Select` a mod drew (`onSelect`): `value` is the
    /// chosen option. `next` runs the handler.
    UI_SELECT: "ui.select", Chain, crate::render::HandlerEvent => (), &["handler", "event"], &["/handler", "/event"], WRAP;
    /// A surface asks for a component (`component`, a site such as `Pane` or
    /// `AbovePrompt`) at `requestId`, sized by `viewport`, with that site's
    /// `props`. Return a view tree to draw it; `next` returns what the rest of
    /// the chain, ending in bingo's own drawing, would draw. Render hooks read
    /// state but never write it.
    UI_RENDER: "ui.render", Chain, crate::render::RenderRequest => crate::View, &["component", "requestId", "surface"], &["/component", "/requestId", "/surface", "/viewport"], WRAP;

    /// A plugin asks to load, naming itself, its version, origin and runtime.
    /// `next` returns the grant trust settings give it; return a narrower grant,
    /// or `{reason}` to refuse the plugin. A grant is never widened here.
    PLUGIN_REGISTER: "plugin.register", Chain, crate::PluginRegistration => crate::PluginDecision, &["name", "origin", "runtime"], &["/name", "/version", "/origin", "/runtime"], WRAP;
    /// Input from a person or client reaches the session, before the kernel
    /// decides turn, queue, steer or deliver. `next` admits it as given; return
    /// `{accepted: input}` with changed input, `{rejected: error}` to refuse it,
    /// or `{redirect: {session, input}}` to send it to another session.
    PROMPT_SUBMIT: "prompt.submit", Chain, crate::Input => crate::SubmitResult, &[], &[], PASS;
    /// Permission for a tool call, before it runs: `tool`, `tool_use_id` and
    /// the tool's own arguments beside them. `next` returns bingo's decision
    /// (settings, mode, or asking the person) with the call; a mod may tighten
    /// it, and only `full` trust may allow.
    TOOL_CHECK: "tool.check", Chain, ToolCall => crate::CheckResult, &["tool"], &["/tool", "/tool_use_id"], PASS;
    /// A tool call runs: `tool`, `tool_use_id` and the tool's own arguments
    /// beside them, not under `input`. `next` runs the tool and returns its
    /// output; return output to answer without running it, `{deny: reason}` to
    /// refuse, or call `next` with changed arguments to rewrite them.
    TOOL_CALL: "tool.call", Chain, ToolCall => ToolCallResult, &["tool"], &["/tool", "/tool_use_id"], WRAP;
    /// A completion a plugin asked of the host through `$.model.complete`;
    /// `plugin` names the asker. `next` calls the model; return model events
    /// to answer instead.
    MODEL_COMPLETE: "model.complete", Chain, Complete => Vec<crate::ModelEvent>, &["plugin"], &["/plugin"], WRAP;
    /// One model call inside a turn. `next` sends the request to the
    /// configured provider and returns its events or a provider error; return
    /// events to answer without calling the model.
    TURN_STEP: "turn.step", Chain, crate::ModelRequest => Result<Vec<crate::ModelEvent>, crate::ProviderError>, &[], &[], WRAP, true;
    /// The model request a turn is about to send: system text, messages and
    /// tools. `next` returns the request bingo composed; return a changed one
    /// to add, remove or reorder context.
    PROMPT_COMPOSE: "prompt.compose", Chain, crate::ModelRequest => crate::ModelRequest, &[], &[], WRAP, true;
    /// Compaction of a session's history: the `items`, the context `usage`,
    /// the `request`, the `reason` and earlier `failures`. `next` runs bingo's
    /// summarizing compaction; return a compaction or error to replace it.
    SESSION_COMPACT: "session.compact", Chain, crate::CompactInput => Result<crate::Compaction, crate::CompactError>, &[], &[], WRAP;
    /// The model has stopped calling tools and the turn would end. `next`
    /// returns `"complete"` unless a hook asks to go on; return
    /// `{continue: {origin, reason}}` to keep the turn running, with `reason`
    /// given back to the model.
    TURN_STOP: "turn.stop", Chain, () => crate::StopResult, &[], &[], PASS;
    /// A tool as the model will see it: name, description and input schema.
    /// `next` returns the spec bingo has; return a changed spec to reword it,
    /// or `null` to hide the tool from the model.
    TOOL_DESCRIBE: "tool.describe", Chain, crate::ToolSpec => Option<crate::ToolSpec>, &["name"], &["/name"], WRAP, true;
    /// Completions offered while the person types, by `kind` (such as
    /// `commands` or `models`). `next` returns bingo's catalog; return a changed
    /// catalog to add, remove or reorder entries.
    PROMPT_AUTOCOMPLETE: "prompt.autocomplete", Chain, crate::Catalog => crate::Catalog, &["kind"], &["/kind"], WRAP;
    /// Input one session sends to another: `to` is the target and `delivery`
    /// says whether it wakes the target or waits. `next` delivers it; rewrite
    /// the input, or deny to stop it.
    SESSION_SEND: "session.send", Chain, Send => (), &["to"], &[], PASS;
    /// Input reaching this session from elsewhere, such as a channel or another
    /// session, before it is queued. `next` returns it as received; rewrite it,
    /// or deny to drop it.
    SESSION_RECEIVE: "session.receive", Chain, crate::Input => crate::Input, &["origin.surface"], &[], PASS;
    /// A record a plugin appends to the session's history: `plugin` owns it,
    /// `kind` and `payload` are its own. `next` appends it; only the owning
    /// plugin may rewrite it, others observe or deny.
    SESSION_APPEND: "session.append", Chain, Append => Append, &["plugin", "kind"], &["/plugin", "/kind"], PASS;
    /// A turn has ended with its `outcome`, `usage` and final `answer`. `next`
    /// returns the completion text bingo composes from that (or `null` for
    /// none); return other text to replace it.
    TURN_COMPLETE: "turn.complete", Chain, Turn => Option<String>, &[], &["/turn", "/outcome", "/usage", "/answer"], WRAP;
    /// A slash command runs: `command` is its name without the slash and
    /// `args` the rest of the line. `next` runs the command's own handler;
    /// return `{text}` or an outcome to answer it yourself.
    COMMAND_RUN: "command.run", Chain, CommandRun => CommandReply, &["command"], &["/command"], WRAP;
    /// A command as the palette and completions list it. `next` returns the
    /// spec bingo has; return a changed spec, or `null` to hide the command.
    COMMAND_DESCRIBE: "command.describe", Chain, crate::CommandSpec => Option<crate::CommandSpec>, &["name"], &["/name"], WRAP;
    /// A setting changes: `key` and its new `value`. `next` stores it; rewrite
    /// the value, or deny the change.
    CONFIG_SET: "config.set", Chain, ConfigSet => ConfigSet, &["key"], &["/key"], PASS;
    /// The settings a client sees: the kernel's keys and each plugin's slice.
    /// `next` returns bingo's view; return a changed view.
    CONFIG_DESCRIBE: "config.describe", Chain, crate::ConfigView => crate::ConfigView, &[], &[], WRAP;
    /// Observation: a turn opened, with the input items it took and its
    /// origin. A returned value is ignored.
    TURN_START: "turn.start", Observation, TurnStart => (), &[], &[], WATCH;
    /// Observation: once per model round, the usage so far and the context
    /// reading. A returned value is ignored.
    SESSION_MEASURE: "session.measure", Observation, Measure => (), &[], &[], WATCH;
    /// Observation: a session opened, and again for a mod after it reloads. A
    /// returned value is ignored.
    SESSION_START: "session.start", Observation, Session => (), &[], &[], WATCH;
    /// Observation: a session closed. A returned value is ignored.
    SESSION_END: "session.end", Observation, Session => (), &[], &[], WATCH;
    /// Observation: a client, such as a terminal or chat surface, attached to
    /// the session. A returned value is ignored.
    SESSION_ATTACH: "session.attach", Observation, Attachment => (), &["client.surface"], &[], WATCH;
    /// Observation: a client detached from the session. A returned value is
    /// ignored.
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
                    "shapesRequest":entry.shapes_request, "description":entry.description(),
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
            surface: None,
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
        let held = COMMAND_RUN.pinning(&["/command", "/args"]);
        assert_eq!(held, *COMMAND_RUN);
        assert_eq!(held.pinned, &["/command", "/args"]);
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
