//! Arguments to existing host doors, independent of the JavaScript transport.
use bingo_sdk::*;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::Value;

macro_rules! params {
    ($($name:ident { $($field:ident : $type:ty),* $(,)? })*) => {$ (
        #[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
        #[serde(rename_all = "camelCase")]
        pub struct $name { $(pub $field: $type),* }
    )*};
}
params! {
    Read { session: SessionId, key: String }
    Setting { session: SessionId, key: String, value: Value }
    Submit { session: SessionId, input: Input }
    Invoke { session: SessionId, call: ToolCall }
    Run { session: SessionId, name: String, args: Value }
    Messages { session: SessionId, page: HistoryPage }
    Authorize { session: SessionId, kind: InteractionKind, answers: Vec<AnswerSpec> }
    RegisterTool { session: SessionId, spec: ToolSpec, callback: String }
    RegisterCommand { session: SessionId, spec: CommandSpec, callback: String }
    Millis { ms: u64 }
    Timer { session: SessionId, ms: u64, callback: String }
    Classify { session: SessionId, turn: Option<TurnId>, text: String, labels: Vec<String> }
    Fork { session: SessionId, turn: Option<TurnId>, prompt: String }
    UsageView { session: Usage, context: Option<ContextUsage>, last_turn: Option<Usage> }
    PromptView { queue: Vec<QueueEntry>, last: Option<String> }
}
