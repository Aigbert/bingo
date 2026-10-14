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
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub place: Option<crate::slot::Place>,
    pub columns: u16,
    pub rows: u16,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct RenderRequest {
    pub component: Component,
    pub request_id: String,
    pub surface: SurfaceName,
    pub props: Value,
    pub viewport: Viewport,
}
impl Component {
    pub fn sealed(self) -> bool {
        matches!(self, Self::Permission | Self::SignIn)
    }

    pub const fn surfaces(self) -> &'static [SurfaceName] {
        match self {
            Self::Browse | Self::Palette | Self::Plugins | Self::Sessions | Self::Status => {
                &[SurfaceName::Terminal, SurfaceName::Desktop]
            }
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

    pub const ALL: [Self; 35] = [
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
    pub handler: String,
    pub event: String,
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
                    Component::Browse
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
    pub handled: bool,
}
