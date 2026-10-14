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
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum SurfaceName {
    Terminal,
    Desktop,
    Text,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct Viewport {
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
    pub const ALL: [Self; 15] = [
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
    fn question_is_named_for_its_drawing() {
        let wire = serde_json::json!("Question");
        let component: Component = serde_json::from_value(wire.clone()).unwrap();
        assert_eq!(serde_json::to_value(component).unwrap(), wire);
        assert!(serde_json::from_value::<Component>(serde_json::json!("AskUserQuestion")).is_err());
    }

    #[test]
    fn render_request_round_trips_and_permission_is_not_a_component() {
        let wire = serde_json::json!({"component":"Pane","requestId":"counter","surface":"terminal",
            "props":{},"viewport":{"columns":80,"rows":8}});
        let request: RenderRequest = serde_json::from_value(wire.clone()).unwrap();
        assert_eq!(serde_json::to_value(&request).unwrap(), wire);
        assert_eq!(request.engine().fold(), "");
        assert!(serde_json::from_value::<Component>(serde_json::json!("Permission")).is_err());
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
