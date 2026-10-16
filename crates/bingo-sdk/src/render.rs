//! The drawing request shared by all surfaces and runtimes.
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::Value;

mod matched;
pub use matched::{Matched, matched};

#[derive(
    Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, JsonSchema,
)]
pub enum Component {
    KeyBar,
    Composer,
    ModelPicker,
    Browse,
    Plugins,
    Palette,
    Sessions,
    Status,
    Question,
    UserMessage,
    AssistantMessage,
    ToolUse,
    ToolResult,
    ToolGroup,
    ToolProgress,
    CommandOutput,
    Spinner,
    TurnDuration,
    InfoNotice,
    SessionMode,
    PromptHint,
    AbovePrompt,
    Pane,
    Permission,
    SignIn,
    Reasoning,
    Compaction,
    Rewind,
    Interruption,
    QuestionAnswer,
    PermissionReceipt,
    Asset,
    ShellRun,
    Action,
    TurnProgress,
    SessionStatus,
    Toast,
    Welcome,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum SurfaceName {
    Terminal,
    Desktop,
    Text,
    Chat,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct Viewport {
    /// Where a pane sits, when the site is a pane.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub place: Option<crate::slot::Place>,
    /// Width in cells.
    pub columns: u16,
    /// Height in cells.
    pub rows: u16,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct RenderRequest {
    /// The site asked for.
    pub component: Component,
    /// Which instance of the site; for `Pane`, the id given to `$.ui.open`.
    pub request_id: String,
    /// The asking surface.
    pub surface: SurfaceName,
    /// The site's facts; their shape is the component's props.
    pub props: Value,
    /// The cells the surface offers the drawing.
    pub viewport: Viewport,
}
impl Component {
    /// What this site draws, for authors choosing a `ui.render` match.
    pub const fn summary(self) -> &'static str {
        match self {
            Self::KeyBar => {
                "The row under the prompt naming what the keys do now, with a toast at its left end."
            }
            Self::Composer => "The prompt line's tag and placeholder around the person's draft.",
            Self::ModelPicker => "The model picker: providers and their models to choose from.",
            Self::Browse => "The transcript reader: the session turn by turn, with search.",
            Self::Plugins => "The plugin list, with each plugin's state and switch.",
            Self::Palette => {
                "The command palette under the prompt line, filtered by what is typed."
            }
            Self::Sessions => "The list of saved sessions to resume.",
            Self::Status => "The status page: session facts, settings and context usage.",
            Self::Question => "A question the model or a plugin asks the person, with its choices.",
            Self::UserMessage => "A message the person sent, in the transcript.",
            Self::AssistantMessage => "The model's text reply, in the transcript.",
            Self::ToolUse => {
                "One tool call in the transcript: tool, input, and output once done; match `props: {tool}` to draw one tool."
            }
            Self::ToolResult => "A tool call's output, drawn on its own.",
            Self::ToolGroup => "Consecutive tool calls folded into one row.",
            Self::ToolProgress => "A running tool's progress line.",
            Self::CommandOutput => "What a slash command answered, in the transcript.",
            Self::Spinner => "The working indicator while a turn runs.",
            Self::TurnDuration => "The line that says how long a finished turn took.",
            Self::InfoNotice => "A notice in the transcript, such as one a plugin posted.",
            Self::SessionMode => "The session's active modes beside the prompt.",
            Self::PromptHint => "The hint line under the prompt.",
            Self::AbovePrompt => {
                "The band directly above the prompt; one per session, `requestId` `AbovePrompt`."
            }
            Self::Pane => {
                "A pane a plugin opened with `$.ui.open`; `requestId` is the pane's `id`."
            }
            Self::Permission => {
                "The permission question for a tool call. Sealed: only bingo draws it."
            }
            Self::SignIn => "A provider's sign-in flow. Sealed: only bingo draws it.",
            Self::Reasoning => "The model's reasoning text, in the transcript.",
            Self::Compaction => "The record that history was compacted, with its summary.",
            Self::Rewind => "The record that the session was rewound to an earlier turn.",
            Self::Interruption => "The marker where a turn was interrupted.",
            Self::QuestionAnswer => "A question and the answer the person gave, in the transcript.",
            Self::PermissionReceipt => "The record of a permission decision, in the transcript.",
            Self::Asset => "A file or picture the session produced, in the transcript.",
            Self::ShellRun => "A shell command the person ran from the prompt, with its output.",
            Self::Action => "An action a client ran, with its result.",
            Self::TurnProgress => "A running turn's elapsed time and tokens.",
            Self::SessionStatus => "The status line: model, effort, directory and context use.",
            Self::Toast => "A short-lived notification.",
            Self::Welcome => "The welcome card a new session opens with.",
        }
    }

    pub fn sealed(self) -> bool {
        matches!(self, Self::Permission | Self::SignIn)
    }

    pub const fn surfaces(self) -> &'static [SurfaceName] {
        match self {
            Self::KeyBar
            | Self::Composer
            | Self::ModelPicker
            | Self::Browse
            | Self::Palette
            | Self::Plugins
            | Self::Sessions
            | Self::Status => &[SurfaceName::Terminal, SurfaceName::Desktop],
            Self::AssistantMessage | Self::CommandOutput | Self::InfoNotice | Self::ShellRun => &[
                SurfaceName::Terminal,
                SurfaceName::Desktop,
                SurfaceName::Text,
                SurfaceName::Chat,
            ],
            Self::Question
            | Self::UserMessage
            | Self::ToolUse
            | Self::ToolResult
            | Self::ToolGroup
            | Self::ToolProgress
            | Self::Spinner
            | Self::TurnDuration
            | Self::SessionMode
            | Self::PromptHint
            | Self::AbovePrompt
            | Self::Pane
            | Self::Permission
            | Self::SignIn
            | Self::Reasoning
            | Self::Compaction
            | Self::Rewind
            | Self::Interruption
            | Self::QuestionAnswer
            | Self::PermissionReceipt
            | Self::Asset
            | Self::Action
            | Self::TurnProgress
            | Self::SessionStatus
            | Self::Toast
            | Self::Welcome => &[
                SurfaceName::Terminal,
                SurfaceName::Desktop,
                SurfaceName::Text,
            ],
        }
    }

    pub const ALL: [Self; 38] = [
        Self::KeyBar,
        Self::Composer,
        Self::ModelPicker,
        Self::Browse,
        Self::Plugins,
        Self::Palette,
        Self::Sessions,
        Self::Status,
        Self::Question,
        Self::UserMessage,
        Self::AssistantMessage,
        Self::ToolUse,
        Self::ToolResult,
        Self::ToolGroup,
        Self::ToolProgress,
        Self::CommandOutput,
        Self::Spinner,
        Self::TurnDuration,
        Self::InfoNotice,
        Self::SessionMode,
        Self::PromptHint,
        Self::AbovePrompt,
        Self::Pane,
        Self::Permission,
        Self::SignIn,
        Self::Reasoning,
        Self::Compaction,
        Self::Rewind,
        Self::Interruption,
        Self::QuestionAnswer,
        Self::PermissionReceipt,
        Self::Asset,
        Self::ShellRun,
        Self::Action,
        Self::TurnProgress,
        Self::SessionStatus,
        Self::Toast,
        Self::Welcome,
    ];
}
impl RenderRequest {
    pub fn engine(&self) -> crate::View {
        crate::View::Engine {
            component: self.component,
            props: self.props.clone(),
        }
    }
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct Invalidate {
    pub session: crate::SessionId,
    pub requests: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct HandlerEvent {
    /// The token a drawn handler became, or a pane id for `close`.
    pub handler: String,
    /// `press`, `input`, `select` or `close`.
    pub event: String,
    /// What the surface sent: the text, the chosen value, or the key.
    #[serde(default)]
    pub value: Value,
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn a_pull_is_completed_by_each_asking_surface() {
        let wire = serde_json::json!({"component":"Pane","requestId":"one","props":{},"viewport":{"columns":0,"rows":0}});
        let pull: PullRequest = serde_json::from_value(wire.clone()).unwrap();
        assert_eq!(serde_json::to_value(&pull).unwrap(), wire);
        for surface in Component::Pane.surfaces() {
            assert_eq!(pull.on(*surface).surface, *surface);
            assert_eq!(pull.on(*surface).props, pull.props);
        }
    }
    #[test]
    fn every_component_names_its_surfaces() {
        for component in Component::ALL {
            assert!(component.surfaces().contains(&SurfaceName::Terminal));
            assert!(component.surfaces().contains(&SurfaceName::Desktop));
            assert_eq!(
                component.surfaces().contains(&SurfaceName::Text),
                !matches!(
                    component,
                    Component::KeyBar
                        | Component::Composer
                        | Component::ModelPicker
                        | Component::Browse
                        | Component::Palette
                        | Component::Plugins
                        | Component::Sessions
                        | Component::Status
                )
            );
            assert_eq!(
                component.surfaces().contains(&SurfaceName::Chat),
                matches!(
                    component,
                    Component::AssistantMessage
                        | Component::CommandOutput
                        | Component::InfoNotice
                        | Component::ShellRun
                )
            );
        }
    }
    #[test]
    fn pane_viewport_keeps_its_seated_place() {
        let wire = serde_json::json!({"columns":64,"rows":24,"place":"side"});
        let viewport: Viewport = serde_json::from_value(wire.clone()).unwrap();
        assert_eq!(serde_json::to_value(viewport).unwrap(), wire);
    }

    #[test]
    fn question_is_named_for_its_drawing() {
        let wire = serde_json::json!("Question");
        let component: Component = serde_json::from_value(wire.clone()).unwrap();
        assert_eq!(serde_json::to_value(component).unwrap(), wire);
        assert!(serde_json::from_value::<Component>(serde_json::json!("AskUserQuestion")).is_err());
    }

    #[test]
    fn render_request_round_trips() {
        let wire = serde_json::json!({"component":"Pane","requestId":"counter","surface":"terminal",
            "props":{},"viewport":{"columns":80,"rows":8}});
        let request: RenderRequest = serde_json::from_value(wire.clone()).unwrap();
        assert_eq!(serde_json::to_value(&request).unwrap(), wire);
        assert_eq!(request.engine().fold(), "");
        assert!(Component::Permission.sealed());
        assert!(Component::SignIn.sealed());
        assert!(!Component::Question.sealed());
        assert_eq!(Component::Permission.surfaces().len(), 3);
    }
}

tokio::task_local! { static PERSON_INPUT: bool; }
/// The core binds this at the input boundary; a mod cannot set it through JSON.
pub fn asked() -> bool {
    PERSON_INPUT.try_with(|asked| *asked).unwrap_or(false)
}
pub async fn under_input<T>(asked: bool, future: impl std::future::Future<Output = T>) -> T {
    PERSON_INPUT.scope(asked, future).await
}

tokio::task_local! { static VOUCHED_PRESS: bool; }
/// Whether the key this run answers was pressed on a drawing the running
/// built-in drew itself and reached it as pressed. The core binds this; a
/// mod cannot set it through JSON.
pub fn vouched() -> bool {
    VOUCHED_PRESS.try_with(|vouched| *vouched).unwrap_or(false)
}
pub async fn under_press<T>(vouched: bool, future: impl std::future::Future<Output = T>) -> T {
    VOUCHED_PRESS.scope(vouched, future).await
}

/// A placement has no asking surface until a client pulls it.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct PullRequest {
    pub component: Component,
    pub request_id: String,
    pub props: Value,
    pub viewport: Viewport,
}
impl PullRequest {
    pub fn on(&self, surface: SurfaceName) -> RenderRequest {
        RenderRequest {
            component: self.component,
            request_id: self.request_id.clone(),
            surface,
            props: self.props.clone(),
            viewport: self.viewport.clone(),
        }
    }
}
impl From<RenderRequest> for PullRequest {
    fn from(request: RenderRequest) -> Self {
        Self {
            component: request.component,
            request_id: request.request_id,
            props: request.props,
            viewport: request.viewport,
        }
    }
}

/// A control may return the key to the asking surface.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct HandlerResult {
    /// `false` returns the key to the surface.
    pub handled: bool,
}
