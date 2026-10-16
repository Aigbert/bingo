//! Arguments to existing host doors, independent of the JavaScript transport.
use bingo_sdk::*;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::Value;

macro_rules! params {
    ($($name:ident { $($(#[doc = $doc:literal])* $field:ident : $type:ty),* $(,)? })*) => {$ (
        #[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
        #[serde(rename_all = "camelCase")]
        pub struct $name { $($(#[doc = $doc])* pub $field: $type),* }
    )*};
}
params! {
    Read {
        session: SessionId,
        /// The setting's dotted key.
        key: String,
    }
    Setting {
        session: SessionId,
        /// The setting's dotted key.
        key: String,
        /// Its new value.
        value: Value,
    }
    Submit {
        session: SessionId,
        /// What to submit, as a person would type it.
        input: Input,
    }
    Invoke {
        session: SessionId,
        /// The tool and its arguments.
        call: ToolCall,
    }
    Run {
        session: SessionId,
        /// The command's name, without the slash.
        name: String,
        /// Its arguments.
        args: Value,
    }
    Messages {
        session: SessionId,
        /// Which page of history.
        page: HistoryPage,
    }
    Authorize {
        session: SessionId,
        /// What kind of question it is.
        kind: InteractionKind,
        /// The answers the person may give.
        answers: Vec<AnswerSpec>,
    }
    RegisterTool {
        session: SessionId,
        spec: ToolSpec,
        callback: String,
    }
    RegisterCommand {
        session: SessionId,
        spec: CommandSpec,
        callback: String,
    }
    Millis {
        /// Milliseconds.
        ms: u64,
    }
    Timer {
        session: SessionId,
        /// Milliseconds.
        ms: u64,
        callback: String,
    }
    Classify {
        session: SessionId,
        turn: Option<TurnId>,
        /// The text to classify.
        text: String,
        /// The labels the model chooses from.
        labels: Vec<String>,
    }
    Fork {
        session: SessionId,
        turn: Option<TurnId>,
        /// The one-off question.
        prompt: String,
    }
    UsageView {
        /// Tokens used by the whole session.
        session: Usage,
        /// The current context window reading, once known.
        context: Option<ContextUsage>,
        /// Tokens used by the last turn.
        last_turn: Option<Usage>,
    }
    PromptView {
        /// Inputs waiting for the next turn.
        queue: Vec<QueueEntry>,
        /// The last submitted line.
        last: Option<String>,
    }
}
