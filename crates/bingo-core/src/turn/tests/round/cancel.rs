use super::*;

struct HeldReceipt {
    recorded: RecordingHost,
    entered: mpsc::UnboundedSender<()>,
}
#[async_trait]
impl TurnHost for HeldReceipt {
    fn offered(&self, _: Vec<Arc<dyn Tool>>) {}
    fn emit(&self, event: Event) {
        self.recorded.emit(event);
    }
    async fn ask(
        &self,
        item: Option<ItemId>,
        kind: InteractionKind,
        answers: Vec<AnswerSpec>,
    ) -> Result<crate::gate::Answered, KernelError> {
        self.recorded.ask(item, kind, answers).await
    }
    async fn absorb(&self) -> Vec<(IntentId, Input)> {
        vec![]
    }
    async fn receipt(&self, _: Item) -> Result<Item, KernelError> {
        self.entered.send(()).unwrap();
        std::future::pending().await
    }
}

#[tokio::test]
async fn esc_drops_a_receipt_wait_before_any_later_call_is_decided() {
    let seen = Arc::new(Mutex::new(Vec::new()));
    let provider = ScriptedProvider::new(vec![Script::Events(calls(&["Echo", "Echo"]))]);
    let mut cfg = config(provider, vec![Arc::new(EchoTool { read_only: false })]);
    cfg.hooks = HookSet::fixed(vec![OrderingHook::new("check", &seen)]);
    let (entered, mut receipt) = mpsc::unbounded_channel();
    let host = HeldReceipt {
        recorded: RecordingHost::new(),
        entered,
    };
    host.recorded
        .answers
        .lock()
        .unwrap()
        .push_back(Answer::AllowOnce);
    let cancel = CancellationToken::new();
    let work = run_turn(
        &cfg,
        TurnRun::new(
            TurnId::from_raw("trn_1"),
            &transcript(&history("go")),
            cancel.clone(),
            TurnKind::Respond,
        ),
        &host,
    );
    let (out, ()) = tokio::join!(work, async {
        receipt.recv().await.unwrap();
        cancel.cancel();
    });
    assert!(matches!(out.status, TurnStatus::Interrupted { .. }));
    assert_eq!(seen.lock().unwrap().len(), 1);
    assert!(
        out.items
            .iter()
            .all(|item| !matches!(item.body, ItemBody::PermissionReceipt { .. }))
    );
    assert!(!host.recorded.events().iter().any(|event| matches!(event,
        Event::ItemUpdated { item } if item.status == ItemStatus::Running)));
}

struct HeldAfter(mpsc::UnboundedSender<()>);
#[async_trait]
impl Hook for HeldAfter {
    fn id(&self) -> &str {
        "held-after"
    }
    fn matcher(&self) -> HookMatcher {
        HookMatcher {
            points: vec![HookPoint::AfterTool],
            tool: None,
        }
    }
    async fn after_tool(&self, _: &ToolCall, _: &ToolOutput, _: &HookContext) -> HookOutcome {
        self.0.send(()).unwrap();
        std::future::pending().await
    }
}

#[tokio::test]
async fn esc_drops_after_hooks_and_keeps_calls_that_landed() {
    let (entered, mut after) = mpsc::unbounded_channel();
    let mut cfg = config(
        ScriptedProvider::new(vec![Script::Events(calls(&["Echo", "Echo"]))]),
        vec![Arc::new(EchoTool { read_only: true })],
    );
    cfg.hooks = HookSet::fixed(vec![Arc::new(HeldAfter(entered))]);
    let host = RecordingHost::new();
    let cancel = CancellationToken::new();
    let work = run(&cfg, &host, cancel.clone());
    let (out, ()) = tokio::join!(work, async {
        after.recv().await.unwrap();
        cancel.cancel();
    });
    assert!(matches!(out.status, TurnStatus::Interrupted { .. }));
    assert_eq!(
        out.items
            .iter()
            .filter(|item| matches!(item.body, ItemBody::ToolCall { .. })
                && item.status == ItemStatus::Completed)
            .count(),
        2
    );
    assert!(after.try_recv().is_err());
}
