use bingo_sdk::{
    ToolLook,
    slot::{self, Slot},
    *,
};
use serde_json::{Value, json};

pub struct Progress;
#[async_trait::async_trait]
impl Tool for Progress {
    fn spec(&self) -> ToolSpec {
        ToolSpec {
            look: Some(ToolLook::new('#', "progress")),
            name: "HelloProgress".into(),
            description: "Publish one fixture progress frame".into(),
            input_schema: json!({"type":"object","properties":{"value":{"type":"integer","minimum":0,"maximum":100}},"required":["value"],"additionalProperties":false}),
            meta: Default::default(),
        }
    }
    fn traits(&self, _: &Value) -> ToolTraits {
        ToolTraits {
            read_only: true,
            trusted: true,
            concurrency_safe: false,
            ..Default::default()
        }
    }
    async fn call(&self, input: Value, cx: &ToolContext) -> Result<ToolOutput, ToolError> {
        let value = input["value"]
            .as_u64()
            .filter(|value| *value <= 100)
            .ok_or_else(|| ToolError::InvalidInput("value must be between 0 and 100".into()))?;
        let view = View::Progress {
            value,
            total: Some(100),
            label: Some("cargo test".into()),
        };
        cx.host
            .signal(
                &cx.session,
                "hello",
                "progress",
                slot::with(
                    json!(view),
                    [Slot::Activity {
                        verb: None,
                        view: view.clone().into(),
                    }],
                ),
            )
            .await
            .map_err(|error| ToolError::Failed(error.message))?;
        let mut output = ToolOutput::text("Published one progress frame.");
        output.display = Some(view);
        Ok(output)
    }
}

#[cfg(test)]
mod look_tests {
    use super::*;
    #[test]
    fn every_tool_declares_its_look() {
        assert_eq!(Progress.spec().look, Some(ToolLook::new('#', "progress")));
    }
}
