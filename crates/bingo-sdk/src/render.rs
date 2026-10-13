//! The drawing request shared by all surfaces and runtimes.
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(
    Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, JsonSchema,
)]
pub enum Component {
    AskUserQuestion,
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

#[cfg(test)]
mod tests {
    use super::*;
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
