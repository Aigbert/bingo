//! The quiet trait is journaled when a call opens.

use super::*;

struct QuietEcho;

#[async_trait]
impl Tool for QuietEcho {
    fn spec(&self) -> ToolSpec {
        EchoTool { read_only: true }.spec()
    }
    fn traits(&self, input: &serde_json::Value) -> ToolTraits {
        ToolTraits {
            quiet: input["quiet"] == true,
            ..ToolTraits::read_only()
        }
    }
    async fn call(
        &self,
        input: serde_json::Value,
        cx: &ToolContext,
    ) -> Result<ToolOutput, ToolError> {
        EchoTool { read_only: true }.call(input, cx).await
    }
}

#[tokio::test]
async fn quiet_is_stamped_when_opened_and_kept_through_completion() {
    for quiet in [false, true] {
        let provider = ScriptedProvider::new(vec![
            Script::Events(tool_call("Echo", json!({"quiet": quiet, "v": 1}))),
            Script::Events(text("done")),
        ]);
        let cfg = config(provider, vec![Arc::new(QuietEcho)]);
        let host = RecordingHost::new();
        run(&cfg, &host, CancellationToken::new()).await;
        let calls: Vec<_> = host
            .events()
            .into_iter()
            .filter_map(|event| match event {
                Event::ItemStarted { item }
                | Event::ItemUpdated { item }
                | Event::ItemCompleted { item } => match item.body {
                    ItemBody::ToolCall { quiet, .. } => Some(quiet),
                    _ => None,
                },
                _ => None,
            })
            .collect();
        assert_eq!(calls, vec![quiet; 3]);
    }
}
