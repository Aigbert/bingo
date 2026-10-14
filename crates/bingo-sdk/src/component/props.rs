use super::*;

macro_rules! props {
    ($name:ident { $($(#[$meta:meta])* $field:ident: $ty:ty),* $(,)? }) => {
        #[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
        #[serde(rename_all = "camelCase")]
        pub struct $name { $($(#[$meta])* pub $field: $ty),* }
    };
}
props!(QuestionProps {
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
