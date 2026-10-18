use super::*;

struct ScriptedAgent;

#[async_trait]
impl Agent for ScriptedAgent {
    fn id(&self) -> &str {
        "fake"
    }
    async fn models(&self) -> Result<Vec<ModelInfo>, ProviderError> {
        Ok(vec![])
    }
    async fn run(&self, turn: AgentTurn) -> Result<AgentStream, ProviderError> {
        assert_eq!(turn.input, vec![ContentPart::text("hello")]);
        assert!(turn.tools.is_empty());
        let judged = ExternalCall {
            id: "exec-1".into(),
            kind: CallKind::Execute,
            title: "Run tests".into(),
            locations: vec![],
            raw: json!({"command":"test"}),
        };
        assert!(matches!(
            turn.judge.judge(judged).await,
            Verdict::Deny { .. }
        ));
        let call = ExternalCall {
            id: "edit-1".into(),
            kind: CallKind::Edit,
            title: "Edit note.md".into(),
            locations: vec![],
            raw: json!({}),
        };
        Ok(Box::pin(futures::stream::iter(vec![
            Ok(AgentEvent::Call(call.clone())),
            Ok(AgentEvent::Call(call.clone())),
            Ok(AgentEvent::Done {
                call,
                output: ToolOutput::text("written"),
            }),
            Ok(AgentEvent::Said(ModelEvent::Finish {
                usage: Usage::default(),
                finish_reason: FinishReason::unified(UnifiedFinish::Stop),
            })),
        ])))
    }
}

#[tokio::test]
async fn an_agent_journals_one_external_call() {
    let mut cfg = config(ScriptedProvider::new(vec![]), vec![]);
    cfg.model.as_mut().unwrap().endpoint = super::super::Endpoint::Agent(Arc::new(ScriptedAgent));
    let host = RecordingHost::new();
    let out = run(&cfg, &host, CancellationToken::new()).await;
    assert_eq!(out.status, TurnStatus::Completed);
    let completed: Vec<_> = host
        .events()
        .into_iter()
        .filter_map(|e| match e {
            Event::ItemCompleted { item } if matches!(item.body, ItemBody::ToolCall { .. }) => {
                Some(item)
            }
            _ => None,
        })
        .collect();
    let mut stamps = Vec::new();
    for event in host.events() {
        if let Event::ItemStarted { item }
        | Event::ItemUpdated { item }
        | Event::ItemCompleted { item } = event
            && let ItemBody::ToolCall {
                asked_at, began_at, ..
            } = item.body
        {
            assert_eq!(asked_at, None);
            assert_eq!(began_at, Some(item.started_at));
            stamps.push(began_at);
        }
    }
    assert!(stamps.len() >= 3);
    assert!(stamps.iter().all(|stamp| stamp == &stamps[0]));
    assert_eq!(completed.len(), 1);
    assert!(completed[0].external());
    assert!(
        matches!(&completed[0].body, ItemBody::ToolCall { name, output: Some(_), .. } if name == "fake:edit")
    );
}

struct Observed(Mutex<Vec<String>>);
#[async_trait]
impl Hook for Observed {
    fn id(&self) -> &str {
        "observer"
    }
    fn matcher(&self) -> HookMatcher {
        HookMatcher::default()
    }
    async fn before_tool(&self, call: &mut ToolCall, _: &HookContext) -> HookOutcome {
        self.0.lock().unwrap().push(format!("before:{}", call.name));
        HookOutcome::Continue
    }
    async fn after_tool(&self, call: &ToolCall, _: &ToolOutput, _: &HookContext) -> HookOutcome {
        self.0.lock().unwrap().push(format!("after:{}", call.name));
        HookOutcome::Continue
    }
}

#[tokio::test]
async fn external_calls_reach_before_and_after_hooks() {
    let mut cfg = config(ScriptedProvider::new(vec![]), vec![]);
    cfg.model.as_mut().unwrap().endpoint = Endpoint::Agent(Arc::new(ScriptedAgent));
    let observed = Arc::new(Observed(Mutex::new(vec![])));
    cfg.hooks = HookSet::fixed(vec![observed.clone()]);
    let host = RecordingHost::new();
    assert_eq!(
        run(&cfg, &host, CancellationToken::new()).await.status,
        TurnStatus::Completed
    );
    assert_eq!(
        *observed.0.lock().unwrap(),
        ["before:fake:execute", "after:fake:edit"]
    );
}

#[tokio::test]
async fn interrupt_drops_a_waiting_judge() {
    let mut cfg = config(ScriptedProvider::new(vec![]), vec![]);
    cfg.model.as_mut().unwrap().endpoint = Endpoint::Agent(Arc::new(ScriptedAgent));
    let entered = Arc::new(tokio::sync::Notify::new());
    struct Waiting(Arc<tokio::sync::Notify>);
    #[async_trait]
    impl Hook for Waiting {
        fn id(&self) -> &str {
            "waiting"
        }
        fn matcher(&self) -> HookMatcher {
            HookMatcher::default()
        }
        async fn before_tool(&self, _: &mut ToolCall, _: &HookContext) -> HookOutcome {
            self.0.notify_one();
            std::future::pending().await
        }
    }
    cfg.hooks = HookSet::fixed(vec![Arc::new(Waiting(entered.clone()))]);
    let host = RecordingHost::new();
    let cancel = CancellationToken::new();
    let stopper = async {
        entered.notified().await;
        cancel.cancel();
    };
    let (out, ()) = tokio::time::timeout(Duration::from_secs(2), async {
        tokio::join!(run(&cfg, &host, cancel.clone()), stopper)
    })
    .await
    .expect("the judge is dropped within the interrupt bound");
    assert!(matches!(out.status, TurnStatus::Interrupted { .. }));
}

struct Offered;
#[async_trait]
impl Agent for Offered {
    fn id(&self) -> &str {
        "offer"
    }
    async fn models(&self) -> Result<Vec<ModelInfo>, ProviderError> {
        Ok(vec![])
    }
    async fn run(&self, turn: AgentTurn) -> Result<AgentStream, ProviderError> {
        assert_eq!(
            turn.tools
                .iter()
                .map(|s| s.name.as_str())
                .collect::<Vec<_>>(),
            ["Echo"]
        );
        Ok(Box::pin(futures::stream::iter(
            text("done").into_iter().map(|e| e.map(AgentEvent::Said)),
        )))
    }
}
#[tokio::test]
async fn agent_gets_exactly_the_turns_resolved_offer() {
    let mut cfg = config(
        ScriptedProvider::new(vec![]),
        vec![Arc::new(EchoTool { read_only: true }), Arc::new(PanicTool)],
    );
    cfg.tools.only = Some(vec!["Echo".into()]);
    cfg.model.as_mut().unwrap().endpoint = Endpoint::Agent(Arc::new(Offered));
    assert_eq!(
        run(&cfg, &RecordingHost::new(), CancellationToken::new())
            .await
            .status,
        TurnStatus::Completed
    );
}

#[tokio::test]
async fn an_agents_reported_window_can_grow_for_the_next_session() {
    let provider = ScriptedProvider::new(vec![Script::Events(vec![
        Ok(ModelEvent::Context {
            used: 10,
            window: 100_000,
        }),
        Ok(ModelEvent::Finish {
            usage: Usage::default(),
            finish_reason: FinishReason::unified(UnifiedFinish::Stop),
        }),
    ])]);
    let mut cfg = config(provider.clone(), vec![]);
    let model = cfg.model.as_mut().unwrap();
    model.learned.reported("scripted", &model.id, 50_000);
    model.endpoint = Endpoint::Agent(provider);
    assert_eq!(
        run(&cfg, &RecordingHost::new(), CancellationToken::new())
            .await
            .status,
        TurnStatus::Completed
    );
    let model = cfg.model.as_ref().unwrap();
    assert_eq!(model.learned.window("scripted", &model.id), Some(100_000));
}

#[tokio::test]
async fn an_agent_cannot_send_a_model_tool_request() {
    let provider = ScriptedProvider::new(vec![Script::Events(vec![Ok(ModelEvent::ToolCall {
        id: "bad".into(),
        name: "Echo".into(),
        input: "{}".into(),
    })])]);
    let mut cfg = config(provider.clone(), vec![]);
    cfg.model.as_mut().unwrap().endpoint = Endpoint::Agent(provider);
    let out = run(&cfg, &RecordingHost::new(), CancellationToken::new()).await;
    assert!(
        matches!(out.status, TurnStatus::Failed { error } if error.message.contains("external calls must use"))
    );
}

#[tokio::test]
async fn an_after_hook_can_end_an_agents_turn() {
    struct Block;
    #[async_trait]
    impl Hook for Block {
        fn id(&self) -> &str {
            "block"
        }
        fn matcher(&self) -> HookMatcher {
            HookMatcher::default()
        }
        async fn after_tool(&self, _: &ToolCall, _: &ToolOutput, _: &HookContext) -> HookOutcome {
            HookOutcome::Block {
                reason: "stop here".into(),
            }
        }
    }
    let mut cfg = config(ScriptedProvider::new(vec![]), vec![]);
    cfg.model.as_mut().unwrap().endpoint = Endpoint::Agent(Arc::new(ScriptedAgent));
    cfg.hooks = HookSet::fixed(vec![Arc::new(Block)]);
    let out = run(&cfg, &RecordingHost::new(), CancellationToken::new()).await;
    assert!(
        matches!(out.status, TurnStatus::Failed { error } if error.message.contains("AfterTool"))
    );
}
