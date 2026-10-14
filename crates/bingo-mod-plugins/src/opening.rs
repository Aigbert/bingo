use bingo_sdk::*;
use serde_json::{Value, json};
pub struct Opening(pub &'static str);
#[async_trait::async_trait]
impl Interceptor for Opening {
    fn id(&self) -> &str {
        if self.0 == "plugins" {
            "plugins.open"
        } else {
            "mcp.open"
        }
    }
    fn matcher(&self) -> LinkMatcher {
        LinkMatcher {
            prefilter: json!({"command":self.0,"args":""}),
            ..LinkMatcher::at(catalogue::COMMAND_RUN)
        }
    }
    async fn chain(
        &self,
        _: &catalogue::Entry,
        _: &HookContext,
        value: Value,
        next: Next<'_, Value, Value>,
    ) -> Result<Value, KernelError> {
        let command = value["command"].as_str().unwrap_or_default();
        if !matches!(command, "plugins" | "mcp")
            || !value["args"].as_str().unwrap_or_default().trim().is_empty()
        {
            return next.run(value).await;
        }
        let open = command.to_string();
        let plain = next.run(value).await?;
        let fallback = serde_json::from_value::<CommandOutcome>(plain)
            .ok()
            .and_then(|outcome| match outcome {
                CommandOutcome::View { view } => Some(view.fold_markdown()),
                _ => None,
            });
        serde_json::to_value(CommandOutcome::Surface {
            action: Action {
                name: "Plugins".into(),
                args: json!({"open":open}),
            },
            fallback,
        })
        .map_err(crate::error)
    }
}
