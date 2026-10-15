//! Quiet, two ways: a tool's quiet trait is journaled when its call opens,
//! and a prompt the turn adds to keep itself going carries the quiet voice.

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

/// The origins of every person-shaped piece the turn wrote down itself.
fn user_origins(host: &RecordingHost) -> Vec<Origin> {
    host.events()
        .into_iter()
        .filter_map(|event| match event {
            Event::ItemCompleted { item } => match item.body {
                ItemBody::User { origin, .. } => Some(origin),
                _ => None,
            },
            _ => None,
        })
        .collect()
}

#[tokio::test]
async fn a_length_recovery_prompt_is_stamped_quiet() {
    let cut = Script::Events(vec![
        Ok(ModelEvent::TextStart { id: "b".into() }),
        Ok(ModelEvent::TextEnd { id: "b".into() }),
        Ok(ModelEvent::Finish {
            usage: Usage::default(),
            finish_reason: FinishReason::unified(UnifiedFinish::Length),
        }),
    ]);
    let provider = ScriptedProvider::new(vec![cut, Script::Events(text("done"))]);
    let host = RecordingHost::new();
    run(&config(provider, vec![]), &host, CancellationToken::new()).await;
    let origins = user_origins(&host);
    assert_eq!(origins.len(), 1, "{origins:?}");
    assert_eq!(origins[0].surface, KERNEL_SURFACE);
    assert_eq!(origins[0].voice, Some(Voice::Quiet));
}

/// Blocks the first stop with a reason, so the turn runs one more round.
struct OnceMore(Mutex<bool>);

#[async_trait]
impl Hook for OnceMore {
    fn id(&self) -> &str {
        "once-more"
    }
    fn matcher(&self) -> HookMatcher {
        HookMatcher {
            points: vec![HookPoint::Stop],
            tool: None,
        }
    }
    async fn on_stop(&self, _: &HookContext) -> HookOutcome {
        let mut blocked = self.0.lock().unwrap();
        if std::mem::replace(&mut *blocked, true) {
            return HookOutcome::Continue;
        }
        HookOutcome::Block {
            reason: "check the tests".into(),
        }
    }
}

#[tokio::test]
async fn a_stop_hooks_reason_to_continue_is_stamped_quiet() {
    let provider = ScriptedProvider::new(vec![
        Script::Events(text("first")),
        Script::Events(text("second")),
    ]);
    let mut cfg = config(provider, vec![]);
    cfg.hooks = HookSet::fixed(vec![Arc::new(OnceMore(Mutex::new(false)))]);
    let host = RecordingHost::new();
    run(&cfg, &host, CancellationToken::new()).await;
    let origins = user_origins(&host);
    assert_eq!(origins.len(), 1, "{origins:?}");
    assert_eq!(origins[0].surface, "hook:once-more");
    assert_eq!(origins[0].voice, Some(Voice::Quiet));
}
