use super::*;

fn request() -> Completion {
    Completion {
        plugin: "stub".into(),
        request: serde_json::from_value(json!({
            "model":"untrusted-choice", "maxTokens":16, "system":[], "messages":[], "tools":[]
        }))
        .unwrap(),
    }
}

async fn fixture(link: bool) -> (Arc<Host>, Attachment, Arc<ScriptedProvider>) {
    let mut contributions = Vec::new();
    if link {
        contributions.push(Contribution::Interceptor {
            placement: bingo_sdk::plugin::Placement::Outer,
            external: None,
            link: Arc::new(SideCall),
        });
    }
    build(contributions).await
}

async fn build(contributions: Vec<Contribution>) -> (Arc<Host>, Attachment, Arc<ScriptedProvider>) {
    let provider = ScriptedProvider::new(vec![
        Script::Events(text("side")),
        Script::Events(text("main")),
    ]);
    let mut contributions = contributions;
    contributions.push(Contribution::Provider(provider.clone()));
    let mut config = HostConfig::new(env()).with_layer("cli", json!({"model":"m"}));
    config.layers.push(
        crate::settings::Layer::new(
            "user",
            json!({
                "pluginTrust":{"stub":{"tier":"rewrite","uses":["model"]}}
            })
            .as_object()
            .unwrap()
            .clone(),
        )
        .user(),
    );
    let host = Host::build(vec![TestPlugin::boxed(&PROVIDER, contributions)], config)
        .await
        .unwrap();
    let attachment = host
        .open(
            SessionSelector::Create {
                spec: SessionSpec {
                    cwd: env().home,
                    ..Default::default()
                },
            },
            who(),
            OpenOptions::default(),
        )
        .await
        .unwrap();
    (host, attachment, provider)
}

struct SideCall;
#[async_trait]
impl Interceptor for SideCall {
    fn id(&self) -> &str {
        "side"
    }
    fn matcher(&self) -> LinkMatcher {
        LinkMatcher::at(bingo_sdk::catalogue::PROMPT_COMPOSE)
    }
    async fn prompt_compose(
        &self,
        cx: &HookContext,
        input: ModelRequest,
        next: Next<'_, ModelRequest, ModelRequest>,
    ) -> Result<ModelRequest, KernelError> {
        cx.host
            .complete(&cx.session, cx.turn.as_ref(), request())
            .await?;
        next.run(input).await
    }
}

#[tokio::test]
async fn complete_outside_a_turn_charges_the_session_and_resolves_its_model() {
    let (host, attachment, provider) = fixture(false).await;
    let events = host
        .complete(&attachment.session, None, request())
        .await
        .unwrap();
    assert!(matches!(events.last(), Some(ModelEvent::Finish { .. })));
    assert_eq!(provider.requests()[0].model, "m");
    let state = host.session_state(&attachment.session).await.unwrap();
    assert_eq!(state.summary.usage.input_tokens, 10);
    assert!(state.turn.is_none() && state.context.is_none());
    let mut denied = request();
    denied.plugin = "ungranted".into();
    assert_eq!(
        host.complete(&attachment.session, None, denied)
            .await
            .unwrap_err()
            .code,
        ErrorCode::PermissionDenied
    );
    host.shutdown().await;
}

#[tokio::test]
async fn complete_inside_a_turn_is_in_its_terminal_usage() {
    let (host, mut attachment, _) = fixture(true).await;
    attachment.handle.submit(
        IntentId::mint(),
        Input::text("hello", Origin::surface("test")),
    );
    let mut sum = Usage::default();
    let mut charges = 0;
    while let Some(frame) = attachment.events.next().await {
        match frame.event {
            Event::UsageCharged { turn, usage, .. } => {
                assert!(turn.is_some());
                sum.add(usage);
                charges += 1;
            }
            Event::TurnUsage { usage, .. } => sum.add(usage),
            Event::TurnCompleted { usage, .. } => {
                assert_eq!(charges, 1);
                assert_eq!(usage, sum);
                assert_eq!(usage.input_tokens, 20);
                break;
            }
            _ => {}
        }
    }
    host.shutdown().await;
}

struct CountComplete(Arc<std::sync::atomic::AtomicUsize>);
#[async_trait]
impl Interceptor for CountComplete {
    fn id(&self) -> &str {
        "count-complete"
    }
    fn matcher(&self) -> LinkMatcher {
        LinkMatcher::at(bingo_sdk::catalogue::MODEL_COMPLETE)
    }
    async fn chain(
        &self,
        _: &bingo_sdk::catalogue::Entry,
        _: &HookContext,
        mut input: Value,
        next: Next<'_, Value, Value>,
    ) -> Result<Value, KernelError> {
        self.0.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        assert_eq!(input["plugin"], "stub");
        input["request"]["maxTokens"] = json!(8);
        next.run(input).await
    }
}

#[tokio::test]
async fn a_completion_crosses_its_kernel_chain_once() {
    let count = Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let (host, attachment, provider) = build(vec![Contribution::Interceptor {
        placement: bingo_sdk::plugin::Placement::Outer,
        external: None,
        link: Arc::new(CountComplete(count.clone())),
    }])
    .await;
    host.complete(&attachment.session, None, request())
        .await
        .unwrap();
    assert_eq!(count.load(std::sync::atomic::Ordering::SeqCst), 1);
    assert_eq!(provider.requests()[0].max_tokens, 8);
    host.shutdown().await;
}
