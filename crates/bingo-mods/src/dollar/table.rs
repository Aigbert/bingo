//! One row owns a noun method's schemas, grant and dispatch door.
use super::params::*;
use bingo_plugin_host::host::*;
use bingo_sdk::{
    catalogue::{self, Entry},
    *,
};
use schemars::{JsonSchema, Schema, SchemaGenerator};
use serde_json::{Value, json};

type Shape = fn(&mut SchemaGenerator) -> Schema;
#[derive(Clone, Copy)]
pub enum Door {
    Kernel(&'static Entry),
    Face,
}
pub struct Row {
    pub noun: &'static str,
    pub method: &'static str,
    pub door: Door,
    pub unsafe_call: bool,
    pub input: Shape,
    pub result: Shape,
    pub accepts: fn(&Value) -> bool,
}
fn shape<T: JsonSchema>(generator: &mut SchemaGenerator) -> Schema {
    generator.subschema_for::<T>()
}
macro_rules! rows {
    ($($noun:literal, $method:literal, $door:expr, $unsafe:literal, $input:ty => $result:ty;)*) => {
        pub static ROWS: &[Row] = &[$(Row {noun:$noun, method:$method, door:$door, unsafe_call:$unsafe,
            input:shape::<$input>, result:shape::<$result>, accepts:|value| serde_json::from_value::<$result>(value.clone()).is_ok()}),*];
    };
}
use Door::{Face, Kernel};
rows! {
    "session", "messages", Face, false, Messages => HistoryChunk;
    "session", "cwd", Face, false, catalogue::Session => String;
    "session", "root", Face, false, catalogue::Session => String;
    "session", "model", Face, false, catalogue::Session => Option<String>;
    "session", "turns", Face, false, catalogue::Session => Vec<TurnId>;
    "session", "id", Face, false, catalogue::Session => SessionId;
    "session", "surfaces", Face, false, catalogue::Session => Vec<ClientIdentity>;
    "session", "usage", Face, false, catalogue::Session => UsageView;
    "session", "version", Face, false, () => String;
    "session", "compact", Kernel(catalogue::SESSION_COMPACT), false, catalogue::Session => IntentId;
    "session", "send", Kernel(catalogue::SESSION_SEND), false, catalogue::Send => ();
    "session", "append", Kernel(catalogue::SESSION_APPEND), false, Extend => ();
    "session", "authorize", Face, false, Authorize => Answer;
    "turn", "abort", Face, false, catalogue::Session => ();
    "prompt", "submit", Kernel(catalogue::PROMPT_SUBMIT), false, Submit => IntentId;
    "prompt", "read", Face, false, catalogue::Session => PromptView;
    "tool", "list", Face, false, () => Catalog;
    "tool", "call", Kernel(catalogue::TOOL_CALL), false, Invoke => ToolOutput;
    "tool", "register", Face, false, RegisterTool => String;
    "command", "list", Face, false, () => Catalog;
    "command", "run", Kernel(catalogue::COMMAND_RUN), false, Run => IntentId;
    "command", "register", Face, false, RegisterCommand => String;
    "config", "list", Face, false, catalogue::Session => ConfigView;
    "config", "read", Face, false, Read => Value;
    "config", "set", Kernel(catalogue::CONFIG_SET), false, Setting => ();
    "model", "complete", Kernel(catalogue::MODEL_COMPLETE), true, Complete => Completed;
    "model", "classify", Face, true, Classify => Option<String>;
    "model", "fork", Face, true, Fork => Completed;
    "state", "get", Face, false, StateGet => Option<StateValue>;
    "state", "set", Face, false, StateSet => StateValue;
    "store", "get", Face, false, StateGet => Option<StateValue>;
    "store", "set", Face, false, StateSet => StateValue;
    "store", "delete", Face, false, StateGet => bool;
    "store", "keys", Face, false, () => Vec<String>;
    "clock", "now", Face, false, () => String;
    "clock", "sleep", Face, false, Millis => ();
    "clock", "after", Face, false, Timer => String;
    "clock", "every", Face, false, Timer => String;
}

pub fn document() -> Value {
    let mut generator = SchemaGenerator::default();
    let methods: serde_json::Map<String, Value> = ROWS.iter().map(|row| {
        let name = format!("{}.{}", row.noun, row.method);
        let door = match row.door {
            Door::Kernel(entry) => json!({"kernel":entry.name}),
            Door::Face => json!({"face":format!("bingo.plugin.{}",row.noun)}),
        };
        (name, json!({"noun":row.noun, "method":row.method, "door":door,
            "unsafe":row.unsafe_call, "input":(row.input)(&mut generator), "result":(row.result)(&mut generator)}))
    }).collect();
    json!({"$schema":"https://json-schema.org/draft/2020-12/schema", "$defs":generator.take_definitions(true), "methods":methods})
}
