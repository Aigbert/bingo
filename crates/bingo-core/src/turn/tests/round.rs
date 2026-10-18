use super::*;
use tokio::sync::mpsc;

fn calls(names: &[&str]) -> Vec<Result<ModelEvent, ProviderError>> {
    let mut events = Vec::new();
    for (index, name) in names.iter().enumerate() {
        events.push(Ok(ModelEvent::ToolCall {
            id: format!("c{index}"),
            name: (*name).into(),
            input: json!({"v": index}).to_string(),
        }));
    }
    events.push(Ok(ModelEvent::Finish {
        usage: Usage::default(),
        finish_reason: FinishReason::unified(UnifiedFinish::ToolCalls),
    }));
    events
}

struct StopAtAsk {
    recorded: RecordingHost,
    cancel: CancellationToken,
    opened: mpsc::UnboundedSender<()>,
}

#[async_trait]
impl TurnHost for StopAtAsk {
    fn offered(&self, _: Vec<Arc<dyn Tool>>) {}
    fn emit(&self, event: Event) {
        self.recorded.emit(event);
    }
    async fn absorb(&self) -> Vec<(IntentId, Input)> {
        vec![]
    }
    async fn ask(
        &self,
        _: Option<ItemId>,
        kind: InteractionKind,
        answers: Vec<AnswerSpec>,
    ) -> Result<crate::gate::Answered, KernelError> {
        if !self.cancel.is_cancelled() {
            self.opened.send(()).unwrap();
            self.cancel.cancelled().await;
            return Err(KernelError::new(ErrorCode::InteractionClosed, "stopped"));
        }
        self.recorded.ask(None, kind, answers).await
    }
}

#[tokio::test]
async fn esc_during_an_ask_never_decides_or_receipts_a_later_call() {
    let provider = ScriptedProvider::new(vec![Script::Events(calls(&["Echo", "Echo"]))]);
    let mut cfg = config(provider, vec![Arc::new(EchoTool { read_only: false })]);
    let seen = Arc::new(Mutex::new(Vec::new()));
    cfg.hooks = HookSet::fixed(vec![OrderingHook::new("check", &seen)]);
    let cancel = CancellationToken::new();
    let (opened, mut asks) = mpsc::unbounded_channel();
    let host = StopAtAsk {
        recorded: RecordingHost::new(),
        cancel: cancel.clone(),
        opened,
    };
    host.recorded
        .answers
        .lock()
        .unwrap()
        .push_back(Answer::AllowOnce);
    let work = run_turn(
        &cfg,
        TurnRun::new(
            TurnId::from_raw("trn_1"),
            &transcript(&history("hello")),
            cancel.clone(),
            TurnKind::Respond,
        ),
        &host,
    );
    let (out, ()) = tokio::join!(work, async {
        asks.recv().await.unwrap();
        cancel.cancel();
    });
    assert!(matches!(out.status, TurnStatus::Interrupted { .. }));
    assert_eq!(seen.lock().unwrap().len(), 1);
    assert_eq!(
        out.items
            .iter()
            .filter(|i| matches!(i.body, ItemBody::PermissionReceipt { .. }))
            .count(),
        0
    );
    let tools: Vec<_> = out
        .items
        .iter()
        .filter(|i| matches!(i.body, ItemBody::ToolCall { .. }))
        .collect();
    assert_eq!(tools.len(), 2);
    assert!(tools.iter().all(|i| i.status == ItemStatus::Interrupted));
}

mod walk;

mod cancel;
