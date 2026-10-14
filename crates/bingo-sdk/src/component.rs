//! Props are a drawing's input, projected from the fold by its surface.
use crate::render::Component;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::Value;

mod props;
pub use props::*;

macro_rules! components {
    ($($name:ident($props:ty)),+ $(,)?) => {
        #[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
        #[serde(tag = "component", content = "props")]
        pub enum Props { $($name($props)),+ }
        impl Props {
            pub fn component(&self) -> Component {
                match self { $(Self::$name(_) => Component::$name),+ }
            }
            pub fn value(&self) -> Value {
                match self { $(Self::$name(props) => serde_json::to_value(props).unwrap_or_default()),+ }
            }
        }
    };
}
components! {
    Question(QuestionProps), UserMessage(UserMessageProps),
    AssistantMessage(AssistantMessageProps), ToolUse(ToolUseProps),
    ToolResult(ToolResultProps), ToolGroup(ToolGroupProps),
    ToolProgress(ToolProgressProps), CommandOutput(CommandOutputProps),
    Spinner(SpinnerProps), TurnDuration(TurnDurationProps),
    InfoNotice(InfoNoticeProps), SessionMode(SessionModeProps),
    PromptHint(PromptHintProps), AbovePrompt(AbovePromptProps), Pane(PaneProps),
}
impl Props {
    pub fn parse(component: Component, props: Value) -> Result<Self, serde_json::Error> {
        serde_json::from_value(serde_json::json!({"component":component,"props":props}))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn every_component_has_a_golden_prop_contract() {
        let fixtures: Vec<Value> =
            serde_json::from_str(include_str!("component/props.json")).unwrap();
        assert_eq!(fixtures.len(), 15);
        let mut components = std::collections::BTreeSet::new();
        for fixture in fixtures {
            let props: Props = serde_json::from_value(fixture.clone()).unwrap();
            assert!(components.insert(props.component()));
            assert_eq!(serde_json::to_value(&props).unwrap(), fixture);
            assert_eq!(
                Props::parse(props.component(), props.value()).unwrap(),
                props
            );
        }
    }
    #[test]
    fn invalid_props_and_permissions_have_no_component_contract() {
        assert!(Props::parse(Component::ToolUse, serde_json::json!({})).is_err());
        assert!(
            serde_json::from_value::<Props>(
                serde_json::json!({"component":"Permission","props":{}})
            )
            .is_err()
        );
    }
}
