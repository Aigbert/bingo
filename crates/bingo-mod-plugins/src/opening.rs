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
        let run: catalogue::CommandRun =
            serde_json::from_value(value.clone()).map_err(crate::error)?;
        if !opens(&run) {
            return next.run(value).await;
        }
        let open = run.name;
        opened(&open, next.run(value).await?)
    }
}
fn opens(run: &catalogue::CommandRun) -> bool {
    matches!(run.name.as_str(), "plugins" | "mcp")
        && run.args.trim().is_empty()
        && run.surface.is_some_and(render::SurfaceName::shows_sheets)
}
fn opened(open: &str, plain: Value) -> Result<Value, KernelError> {
    let reply: catalogue::CommandReply = serde_json::from_value(plain).map_err(crate::error)?;
    let fallback = match CommandOutcome::from(reply) {
        CommandOutcome::View { view } => Some(view.fold_markdown()),
        CommandOutcome::Applied { message } => message,
        _ => None,
    };
    serde_json::to_value(CommandOutcome::Surface {
        action: Action {
            name: "Plugins".into(),
            args: json!({"open":open}),
        },
        fallback,
    })
    .map_err(crate::error)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bare_mcp_opens_the_mcp_group_and_keeps_its_text() {
        for (args, surface, expected) in [
            ("", Some("terminal"), true),
            ("", Some("desktop"), true),
            ("  ", Some("terminal"), true),
            ("tools server", Some("terminal"), false),
            ("", Some("text"), false),
            ("", Some("chat"), false),
            ("", None, false),
        ] {
            assert_eq!(
                opens(
                    &serde_json::from_value(json!({"command":"mcp","args":args,"surface":surface}))
                        .unwrap()
                ),
                expected
            );
        }
        let result = opened(
            "mcp",
            json!(CommandOutcome::View {
                view: View::text("server list")
            }),
        )
        .unwrap();
        assert_eq!(
            result["action"],
            json!({"name":"Plugins","args":{"open":"mcp"}})
        );
        assert_eq!(result["fallback"], "server list");
    }
}
