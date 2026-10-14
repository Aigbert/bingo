use super::*;

struct AnswerPoints;
#[async_trait]
impl Interceptor for AnswerPoints {
    fn id(&self) -> &str {
        "answer-points"
    }
    fn matcher(&self) -> LinkMatcher {
        LinkMatcher::default()
    }
    async fn turn_step(
        &self,
        _: &HookContext,
        _: ModelRequest,
        _: Next<'_, ModelRequest, Result<StepStream, ProviderError>>,
    ) -> Result<Result<StepStream, ProviderError>, KernelError> {
        Ok(Ok(StepStream::new(Box::pin(futures::stream::iter(text(
            "from the link",
        ))))))
    }
    async fn session_compact(
        &self,
        _: &HookContext,
        input: CompactInput,
        _: Next<'_, CompactInput, Result<Compaction, CompactError>>,
    ) -> Result<Result<Compaction, CompactError>, KernelError> {
        Ok(
            ScriptedCompactor::cut(input.items.last().unwrap().id.as_str(), 9000, 100)
                .map_err(Into::into),
        )
    }
}
fn full(interceptor: Arc<dyn Interceptor>) -> crate::chain::Link {
    crate::chain::Link {
        plugin: "fixture".into(),
        placement: bingo_sdk::plugin::Placement::Outer,
        interceptor,
        external: None,
        tier: Tier::Full,
    }
}
#[tokio::test]
async fn a_full_link_answers_turn_step_without_the_provider() {
    let provider = ScriptedProvider::new(vec![]);
    let mut cfg = config(provider.clone(), vec![]);
    cfg.hooks.links.push(full(Arc::new(AnswerPoints)));
    let result = run(&cfg, &RecordingHost::new(), CancellationToken::new()).await;
    assert_eq!(result.status, TurnStatus::Completed);
    assert!(provider.requests().is_empty());
    assert!(result.items.iter().any(
        |item| matches!(&item.body, ItemBody::Assistant { text, .. } if text == "from the link")
    ));
}
#[tokio::test]
async fn a_full_link_answers_compaction_without_the_strategy() {
    let mut cfg = config(ScriptedProvider::new(vec![]), vec![]);
    let strategy = ScriptedCompactor::new(vec![]);
    cfg.compactor = CompactorSet::fixed(Some(strategy.clone()));
    cfg.hooks.links.push(full(Arc::new(AnswerPoints)));
    let host = RecordingHost::new();
    let result = run_turn(
        &cfg,
        TurnRun::new(
            TurnId::from_raw("t"),
            &transcript(&history("hello")),
            CancellationToken::new(),
            TurnKind::Compact { instructions: None },
        ),
        &host,
    )
    .await;
    assert_eq!(result.status, TurnStatus::Completed);
    assert!(strategy.calls.lock().unwrap().is_empty());
    assert!(
        host.events()
            .iter()
            .any(|event| matches!(event, Event::Compacted { .. }))
    );
}
struct Grow;
#[async_trait]
impl Interceptor for Grow {
    fn id(&self) -> &str {
        "grow"
    }
    fn matcher(&self) -> LinkMatcher {
        LinkMatcher::at(bingo_sdk::catalogue::PROMPT_COMPOSE)
    }
    async fn prompt_compose(
        &self,
        _: &HookContext,
        input: ModelRequest,
        next: Next<'_, ModelRequest, ModelRequest>,
    ) -> Result<ModelRequest, KernelError> {
        let mut request = next.run(input).await?;
        request.system.push(SystemBlock {
            text: "extra ".repeat(2000),
            cache: false,
        });
        Ok(request)
    }
}
#[tokio::test]
async fn the_ruler_measures_the_rewritten_composition() {
    let provider = ScriptedProvider::new(vec![Script::Events(text("done"))]);
    let mut cfg = config(provider.clone(), vec![]);
    cfg.hooks.links.push(full(Arc::new(Grow)));
    let host = RecordingHost::new();
    run(&cfg, &host, CancellationToken::new()).await;
    let request = &provider.requests()[0];
    let expected = estimate_tokens(&request.system, &request.messages, &request.tools);
    assert!(expected > 2000);
    assert!(host.events().iter().any(
        |event| matches!(event, Event::TurnUsage { context, .. } if context.used >= expected)
    ));
}
#[tokio::test]
async fn a_tighten_link_at_turn_step_observes_only() {
    let provider = ScriptedProvider::new(vec![Script::Events(text("from provider"))]);
    let mut cfg = config(provider.clone(), vec![]);
    cfg.hooks.links.push(crate::chain::Link {
        plugin: "fixture".into(),
        placement: bingo_sdk::plugin::Placement::Outer,
        interceptor: Arc::new(AnswerPoints),
        external: Some("untrusted".into()),
        tier: Tier::Tighten,
    });
    let host = RecordingHost::new();
    let result = run(&cfg, &host, CancellationToken::new()).await;
    assert_eq!(result.status, TurnStatus::Completed);
    assert_eq!(provider.requests().len(), 1);
    assert!(host.events().iter().any(|event| matches!(event, Event::ItemCompleted { item } if matches!(&item.body, ItemBody::Notice { code, .. } if code == "INTERCEPTOR_SKIPPED"))));
}

struct SwitchDuringStep {
    off: Arc<std::sync::RwLock<std::collections::BTreeSet<String>>>,
    stops: Arc<std::sync::atomic::AtomicUsize>,
}
#[async_trait]
impl Interceptor for SwitchDuringStep {
    fn id(&self) -> &str {
        "switch"
    }
    fn matcher(&self) -> LinkMatcher {
        LinkMatcher::default()
    }
    async fn turn_step(
        &self,
        _: &HookContext,
        input: ModelRequest,
        next: Next<'_, ModelRequest, Result<StepStream, ProviderError>>,
    ) -> Result<Result<StepStream, ProviderError>, KernelError> {
        self.off.write().unwrap().insert("fixture".into());
        next.run(input).await
    }
    async fn turn_stop(
        &self,
        _: &HookContext,
        input: (),
        next: Next<'_, (), StopResult>,
    ) -> Result<StopResult, KernelError> {
        self.stops.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        next.run(input).await
    }
}
#[tokio::test]
async fn a_switch_during_a_turn_keeps_its_stop_link_until_the_next_turn() {
    let provider = ScriptedProvider::new(vec![
        Script::Events(text("one")),
        Script::Events(text("two")),
    ]);
    let mut cfg = config(provider, vec![]);
    let stops = Arc::new(std::sync::atomic::AtomicUsize::new(0));
    cfg.hooks.links.push(full(Arc::new(SwitchDuringStep {
        off: cfg.hooks.running.clone(),
        stops: stops.clone(),
    })));
    for _ in 0..2 {
        let result = run(&cfg, &RecordingHost::new(), CancellationToken::new()).await;
        assert_eq!(result.status, TurnStatus::Completed);
        assert_eq!(stops.load(std::sync::atomic::Ordering::SeqCst), 1);
    }
}
