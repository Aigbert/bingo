//! One row owns a noun method's schemas, grant and dispatch door.
use super::{params::*, world::*};
use bingo_plugin_host::host::*;
use bingo_sdk::{
    catalogue::{self, Entry},
    *,
};
use schemars::{JsonSchema, Schema, SchemaGenerator};
use serde_json::{Value, json};

#[derive(serde::Serialize, JsonSchema)]
pub struct PluginData {
    pub name: String,
    pub root: std::path::PathBuf,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
}
#[derive(JsonSchema)]
pub struct DollarData {
    pub plugin: PluginData,
}

pub fn data_member(name: &str) -> bool {
    static SCHEMA: std::sync::LazyLock<Value> =
        std::sync::LazyLock::new(|| schemars::schema_for!(DollarData).to_value());
    let mut parts = name.split('.');
    let (Some(noun), Some(field)) = (parts.next(), parts.next()) else {
        return false;
    };
    let noun = &SCHEMA["properties"][noun];
    let schema = noun["$ref"]
        .as_str()
        .and_then(|reference| SCHEMA.pointer(&reference[1..]))
        .unwrap_or(noun);
    schema["properties"].get(field).is_some()
}

type Shape = fn(&mut SchemaGenerator) -> Schema;
#[derive(Clone, Copy)]
pub enum Door {
    Kernel(&'static Entry),
    Face,
    Service(&'static str),
}
pub struct Row {
    pub noun: &'static str,
    pub method: &'static str,
    /// What the method does, from the row's doc comment.
    pub doc: &'static str,
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
    ($($(#[doc = $doc:literal])+ $noun:literal, $method:literal, $door:expr, $unsafe:literal, $input:ty => $result:ty;)*) => {
        pub static ROWS: &[Row] = &[$(Row {noun:$noun, method:$method, doc:concat!($($doc),+), door:$door, unsafe_call:$unsafe,
            input:shape::<$input>, result:shape::<$result>, accepts:|value| serde_json::from_value::<$result>(value.clone()).is_ok()}),*];
    };
}
use Door::{Face, Kernel, Service};
rows! {
    /// Call `tool` on MCP `server` with `args`, through the MCP service.
    "mcp", "call", Service("mcp"), false, Value => ToolOutput;
    /// Connect MCP `server`, through the MCP service.
    "mcp", "connect", Service("mcp"), true, Value => ();
    /// Start a sub-agent with `{args: {prompt, agent?}}`, through the agent service; answers with its result.
    "agent", "spawn", Service("agent"), true, Value => ToolOutput;
    /// The sub-agents of a session, through the agent service.
    "agent", "list", Service("agent"), false, catalogue::Session => Vec<SessionSummary>;
    /// Register an agent `{definition}`, through the agent service.
    "agent", "register", Service("agent"), false, Value => ();
    /// Fetch a URL through the host; the body is text and may be `truncated`.
    "http", "fetch", Face, true, Fetch => Fetched;
    /// Read a variable from the mod's environment overlay, or `null`.
    "env", "get", Face, true, EnvGet => Option<String>;
    /// Set a variable in the mod's environment overlay; processes it starts see it.
    "env", "set", Face, true, EnvSet => ();
    /// Run `argv` to completion and answer with its exit code and output.
    "process", "run", Face, true, ProcessInput => ProcessOutput;
    /// Start `argv` in the background; the author call answers with a handle to read, write or kill.
    "process", "spawn", Face, true, ProcessInput => String;
    /// Wait for a spawned process to exit and read all its output; a process is read once.
    "process", "read", Face, true, ProcessId => ProcessOutput;
    /// Write `body` to a spawned process's stdin; `null` closes it.
    "process", "write", Face, true, ProcessWrite => ();
    /// Kill a spawned process.
    "process", "kill", Face, true, ProcessId => ();
    /// Read a file under a granted root, as UTF-8 text or base64.
    "fs", "read", Face, false, FileRead => String;
    /// Write a file under a granted root, replacing it.
    "fs", "write", Face, false, FileWrite => ();
    /// List the entries of a directory under a granted root, up to `limit`.
    "fs", "list", Face, false, FileList => Vec<String>;
    /// Whether a path exists under a granted root.
    "fs", "exists", Face, false, FilePath => bool;
    /// Size and kind of a path under a granted root.
    "fs", "stat", Face, false, FilePath => FileStat;
    /// Each directory from the session's cwd up to the project root that holds `name`, nearest first.
    "fs", "ancestors", Face, false, Ancestors => Vec<std::path::PathBuf>;
    /// A page of the session's history.
    "session", "messages", Face, false, Messages => HistoryChunk;
    /// The session's working directory.
    "session", "cwd", Face, false, catalogue::Session => String;
    /// The session's project root.
    "session", "root", Face, false, catalogue::Session => String;
    /// The model the session is using, or `null`.
    "session", "model", Face, false, catalogue::Session => Option<String>;
    /// The ids of the session's turns, oldest first.
    "session", "turns", Face, false, catalogue::Session => Vec<TurnId>;
    /// The current session's id.
    "session", "id", Face, false, catalogue::Session => SessionId;
    /// The clients attached to the session.
    "session", "surfaces", Face, false, catalogue::Session => Vec<ClientIdentity>;
    /// Token usage: the whole session, the last turn, and `context`, the current context window reading.
    "session", "usage", Face, false, catalogue::Session => UsageView;
    /// The bingo version running the mod.
    "session", "version", Face, false, () => String;
    /// Ask the session to compact its history; runs `session.compact`.
    "session", "compact", Kernel(catalogue::SESSION_COMPACT), false, catalogue::Session => IntentId;
    /// Send input to another session; runs `session.send`.
    "session", "send", Kernel(catalogue::SESSION_SEND), false, catalogue::Send => ();
    /// Append a record this plugin owns to the session's history; runs `session.append`.
    "session", "append", Kernel(catalogue::SESSION_APPEND), false, Extend => ();
    /// Ask the person a question with the given answer kinds and wait for the answer.
    "session", "authorize", Face, false, Authorize => Answer;
    /// End the session's running turn, as `esc` does.
    "turn", "abort", Face, false, catalogue::Session => ();
    /// Submit input to the session as if typed; runs `prompt.submit`.
    "prompt", "submit", Kernel(catalogue::PROMPT_SUBMIT), false, Submit => IntentId;
    /// The session's queued inputs and the last submitted line.
    "prompt", "read", Face, false, catalogue::Session => PromptView;
    /// Every tool the session offers the model.
    "tool", "list", Face, false, () => Catalog;
    /// Call a tool as the model would; runs `tool.check` and `tool.call`, so permission still applies.
    "tool", "call", Kernel(catalogue::TOOL_CALL), false, Invoke => ToolOutput;
    /// Register a tool the model can call; the author call takes the spec and a handler.
    "tool", "register", Face, false, RegisterTool => String;
    /// Every slash command the session offers.
    "command", "list", Face, false, () => Catalog;
    /// Run a slash command by name; runs `command.run`.
    "command", "run", Kernel(catalogue::COMMAND_RUN), false, Run => IntentId;
    /// Register a slash command; the author call takes the spec and an optional handler.
    "command", "register", Face, false, RegisterCommand => String;
    /// Every setting as the session sees it.
    "config", "list", Face, false, catalogue::Session => ConfigView;
    /// Read one setting by its dotted key.
    "config", "read", Face, false, Read => Value;
    /// Change one setting; runs `config.set`.
    "config", "set", Kernel(catalogue::CONFIG_SET), false, Setting => ();
    /// Ask the host's model for a completion; runs `model.complete`.
    "model", "complete", Kernel(catalogue::MODEL_COMPLETE), true, Complete => Completed;
    /// Ask the model to pick one of `labels` for `text`, or `null`.
    "model", "classify", Face, true, Classify => Option<String>;
    /// Ask the model a one-off `prompt` with the session's context, outside the history.
    "model", "fork", Face, true, Fork => Completed;
    /// Read this plugin's value for `key` in the current session, with its version, or `null`.
    "state", "get", Face, false, StateGet => Option<StateValue>;
    /// Write this plugin's value for `key` in the current session; `ifVersion` refuses a stale write.
    "state", "set", Face, false, StateSet => StateValue;
    /// Read this plugin's global value for `key`, shared across sessions, or `null`.
    "store", "get", Face, false, StateGet => Option<StateValue>;
    /// Write this plugin's global value for `key`; `ifVersion` refuses a stale write.
    "store", "set", Face, false, StateSet => StateValue;
    /// Delete this plugin's global value for `key`; answers whether it existed.
    "store", "delete", Face, false, StateGet => bool;
    /// Every key this plugin has in the global store.
    "store", "keys", Face, false, () => Vec<String>;
    /// The host's time in milliseconds since the epoch; tests can set it.
    "clock", "now", Face, false, () => i64;
    /// Wait `ms` milliseconds on the host clock.
    "clock", "sleep", Face, false, Millis => ();
    /// Run a callback once after `ms` milliseconds; answers its timer id.
    "clock", "after", Face, false, Timer => String;
    /// Run a callback every `ms` milliseconds; answers its timer id.
    "clock", "every", Face, false, Timer => String;
}

impl Row {
    pub fn world(&self) -> bool {
        matches!(
            self.noun,
            "fs" | "http" | "process" | "env" | "mcp" | "agent"
        )
    }
}

pub fn document() -> Value {
    let mut generator = SchemaGenerator::default();
    let data = generator.subschema_for::<DollarData>();
    let methods: serde_json::Map<String, Value> = ROWS.iter().map(|row| {
        let name = format!("{}.{}", row.noun, row.method);
        let door = match row.door {
            Door::Kernel(entry) => json!({"kernel":entry.name}),
            Door::Face => json!({"face":format!("bingo.plugin.{}",row.noun)}),
            Door::Service(key) => json!({"face":key,"owner":true}),
        };
        (name, json!({"noun":row.noun, "method":row.method, "door":door, "description":row.doc.trim(),
            "unsafe":row.unsafe_call, "input":(row.input)(&mut generator), "result":(row.result)(&mut generator)}))
    }).collect();
    json!({"$schema":"https://json-schema.org/draft/2020-12/schema", "$defs":generator.take_definitions(true), "methods":methods, "data":data})
}
