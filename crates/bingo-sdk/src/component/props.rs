use super::*;

macro_rules! props {
    ($name:ident { $($(#[$meta:meta])* $field:ident: $ty:ty),* $(,)? }) => {
        #[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
        #[serde(rename_all = "camelCase")]
        pub struct $name { $($(#[$meta])* pub $field: $ty),* }
    };
}
props!(QuestionProps {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    interaction_id: Option<crate::InteractionId>,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    form: bool,
    tool: String,
    questions: Vec<crate::Question>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    metadata_source: Option<String>,
});
props!(UserMessageProps {
    text: String,
    origin: crate::Origin,
    is_expanded: bool
});
props!(AssistantMessageProps {
    text: String,
    is_first_of_reply: bool
});
props!(ToolUseProps {
    #[serde(rename = "tool_use_id")]
    tool_use_id: String,
    tool: String, input: Value, is_running: bool, is_errored: bool,
    is_interrupted: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    output: Option<Value>,
});
props!(ToolResultProps {
    #[serde(rename = "tool_use_id")]
    tool_use_id: String,
    tool: String,
    output: Value,
    is_errored: bool,
});
props!(ToolGroupProps { calls: Vec<ToolUseProps>, is_active: bool, is_expanded: bool });
props!(ToolProgressProps {
    #[serde(rename = "tool_use_id")]
    tool_use_id: String,
    kind: String,
    hint: String,
});
props!(CommandOutputProps {
    command: String,
    args: String,
    text: String,
    is_errored: bool
});
props!(SpinnerProps { word: String, message: Option<String>, suffix: String, mode: String });
props!(TurnDurationProps {
    word: String,
    duration_ms: u64
});
props!(InfoNoticeProps { text: String, command: Option<String> });
props!(SessionModeProps { modes: Vec<String> });
props!(PromptHintProps {
    is_draft: bool,
    is_working: bool,
    hint: String
});
props!(SiteScroll {
    offset: usize,
    body_rows: usize
});
props!(SiteView {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    agent_id: Option<String>,
});
props!(AbovePromptProps {
    has_survey: bool,
    is_working: bool,
    max_rows: u16,
    body_columns: u16,
    scroll: SiteScroll,
    view: SiteView,
});
props!(PaneProps {
    title: String,
    is_focused: bool,
    body_columns: u16,
    placement: String,
    scroll: SiteScroll,
    view: SiteView,
});
props!(ReasoningProps {
    text: String,
    status: crate::ItemStatus
});
props!(CompactionProps {
    summary: String,
    replaced: u32,
    before: u64,
    after: u64,
    duration_ms: u64
});
props!(RewindProps {
    to_turn: crate::TurnId,
    dropped: u32
});
props!(InterruptionProps { marker: String });
props!(QuestionAnswerProps {
    interaction: crate::InteractionId,
    question: String,
    answer: String
});
props!(PermissionReceiptProps { interaction: crate::InteractionId, tool: String, decision: crate::DecisionKind, feedback: Option<String> });
props!(AssetProps { asset: String, label: Option<String> });
props!(ShellRunProps { command: String, output: String, exit: Option<i32>, cwd: std::path::PathBuf, status: crate::ItemStatus });
props!(ActionProps { name: String, args: Value, result: Option<Value>, status: crate::ItemStatus });
props!(TurnProgressProps {
    elapsed_ms: u64,
    tokens_out: u64,
    interruptible: bool,
    stopping: bool
});
props!(SessionStatusProps {
    title: Option<String>, cwd: String, provider: Option<String>, model: Option<String>, effort: Option<crate::Effort>, context: Option<crate::ContextUsage>, usage: crate::Usage });
props!(ToastProps {
    text: String,
    #[schemars(with = "String")]
    until: jiff::Timestamp,
    alert: bool
});
props!(WelcomeProps { session: crate::SessionSummary, effort: Option<crate::Effort>, modes: Vec<String>, update: Option<String>, resume: Option<crate::SessionSummary> });

/// Status is pulled from the asking session; selection belongs to its owner.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct StatusProps {}
