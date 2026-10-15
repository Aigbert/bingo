use super::*;
use std::sync::atomic::{AtomicBool, Ordering};
use tokio::sync::Notify;

struct PausedProviders {
    armed: AtomicBool,
    release: Notify,
    provider: Arc<ScriptedProvider>,
}

#[async_trait]
impl ProviderSource for PausedProviders {
    fn id(&self) -> &str {
        "test.paused"
    }

    async fn providers(&self) -> Vec<Arc<dyn Provider>> {
        if self.armed.swap(false, Ordering::SeqCst) {
            self.release.notified().await;
        }
        vec![self.provider.clone()]
    }
}

#[tokio::test]
async fn concurrent_model_and_thinking_changes_keep_both() {
    let source = Arc::new(PausedProviders {
        armed: AtomicBool::new(false),
        release: Notify::new(),
        provider: ScriptedProvider::new(vec![]),
    });
    let host = Host::build(
        vec![TestPlugin::boxed(
            &PROVIDER,
            vec![Contribution::Providers(source.clone())],
        )],
        HostConfig::new(env()).with_layer("cli", json!({"model": "m"})),
    )
    .await
    .unwrap();
    let client = commands::Client::open(&host).await;
    let id = &client.state.summary.id;
    source.armed.store(true, Ordering::SeqCst);
    let model = host.reconfigure(
        id,
        SessionChange::Model {
            provider: None,
            model: "m2".into(),
        },
    );
    tokio::pin!(model);
    assert!(futures::poll!(&mut model).is_pending());
    assert!(!source.armed.load(Ordering::SeqCst));
    let thinking = host.reconfigure(id, SessionChange::Thinking(Some(Effort::High)));
    tokio::pin!(thinking);
    assert!(futures::poll!(&mut thinking).is_pending());
    source.release.notify_one();
    let (model, thinking) = tokio::join!(model, thinking);
    model.unwrap();
    thinking.unwrap();
    let live = host.live(id).unwrap();
    assert_eq!(live.spec.model.as_deref(), Some("m2"));
    assert_eq!(live.spec.thinking, Some(Some(Effort::High)));
    let state = host.session_state(id).await.unwrap();
    assert_eq!(state.summary.model.as_deref(), Some("m2"));
    assert_eq!(state.config.kernel["thinking"], json!("high"));
    host.shutdown().await;
}

fn variants(parent: SessionId) -> Vec<SessionSpec> {
    let full = SessionSpec {
        cwd: "/work".into(),
        key: Some("test/spec".into()),
        parent: Some(ParentLink {
            session: parent,
            item: Some(ItemId::mint()),
        }),
        title: Some("A named session".into()),
        driver: Driver::Model,
        provider: Some("scripted".into()),
        model: Some("m2".into()),
        system_extra: Some("Keep this instruction".into()),
        tools: Some(vec!["Echo".into()]),
        thinking: Some(Some(Effort::High)),
    };
    let mut variants = vec![full.clone()];
    for field in 0..9 {
        let mut spec = full.clone();
        match field {
            0 => spec.key = None,
            1 => spec.parent = None,
            2 => spec.title = None,
            3 => spec.provider = None,
            4 => spec.model = None,
            5 => spec.system_extra = None,
            6 => spec.tools = None,
            7 => spec.thinking = None,
            8 => spec.thinking = Some(None),
            _ => unreachable!(),
        }
        variants.push(spec);
    }
    variants.push(SessionSpec {
        driver: Driver::Log,
        ..spec("/work")
    });
    variants
}

async fn opened(host: &Host, spec: SessionSpec) -> Attachment {
    host.open(
        SessionSelector::Create { spec },
        who(),
        OpenOptions::default(),
    )
    .await
    .unwrap()
}

#[tokio::test]
async fn every_spec_field_survives_the_journal_round_trip() {
    let store = Arc::new(crate::journal::MemoryStore::new());
    let host = host_on(store.clone(), ScriptedProvider::new(vec![])).await;
    let parent = opened(
        &host,
        SessionSpec {
            provider: Some("scripted".into()),
            model: Some("m2".into()),
            thinking: Some(Some(Effort::Low)),
            ..spec("/work")
        },
    )
    .await;
    host.reconfigure(
        &parent.session,
        SessionChange::Model {
            provider: None,
            model: "m3".into(),
        },
    )
    .await
    .unwrap();
    for spec in variants(parent.session) {
        assert_round_trip(&host, &store, spec).await;
    }
    host.shutdown().await;
}

async fn assert_round_trip(host: &Host, store: &crate::journal::MemoryStore, spec: SessionSpec) {
    let inherits_model = spec.parent.is_some() && spec.model.is_none();
    let inherits_level = spec.parent.is_some() && spec.thinking.is_none();
    let a = opened(host, spec).await;
    if inherits_model {
        assert_eq!(a.snapshot.summary.model.as_deref(), Some("m3"));
    }
    if inherits_level {
        assert_eq!(
            host.session_thinking(&a.session).unwrap(),
            Some(Effort::Low)
        );
    }
    let mut expected = host.live(&a.session).unwrap().spec;
    expected.provider = a.snapshot.summary.provider.clone();
    expected.model = a.snapshot.summary.model.clone();
    close_session(host, &a.session).await;
    let frames = store.replay(&a.session, Seq::ZERO).await.unwrap();
    let folded = session::replayed(&frames).unwrap();
    assert_eq!(crate::spec::spec_of(&folded), expected);
    let b = host
        .open(
            SessionSelector::ById { id: a.session },
            who(),
            OpenOptions::default(),
        )
        .await
        .unwrap();
    assert_eq!(host.live(&b.session).unwrap().spec, expected);
    assert_eq!(folded.summary.system_extra, expected.system_extra);
    assert_eq!(folded.summary.tools, expected.tools);
    close_session(host, &b.session).await;
}

async fn close_session(host: &Host, id: &SessionId) {
    let live = host.lock().remove(id).unwrap();
    live.mailbox.close(CloseReason::Client);
    live.mailbox.wait_closed().await;
}

async fn host_at_level(store: Arc<crate::journal::MemoryStore>, level: Effort) -> Arc<Host> {
    Host::build(
        vec![
            TestPlugin::boxed(
                &PROVIDER,
                vec![Contribution::Provider(ScriptedProvider::new(vec![]))],
            ),
            TestPlugin::boxed(&STORE, vec![Contribution::Store(store)]),
        ],
        HostConfig::new(env()).with_layer("cli", json!({"model": "m", "thinking": level})),
    )
    .await
    .unwrap()
}

#[tokio::test]
async fn resume_prefers_the_fold_level_and_defaults_only_when_absent() {
    let store = Arc::new(crate::journal::MemoryStore::new());
    let first = host_at_level(store.clone(), Effort::Low).await;
    let mut client = commands::Client::open(&first).await;
    assert!(matches!(
        client.ack("/think high").await.0,
        IntentOutcome::Applied { .. }
    ));
    let id = client.state.summary.id.clone();
    first.shutdown().await;
    let next = host_at_level(store.clone(), Effort::Max).await;
    let resumed = next
        .open(SessionSelector::ById { id }, who(), OpenOptions::default())
        .await
        .unwrap();
    assert_eq!(
        next.session_thinking(&resumed.session).unwrap(),
        Some(Effort::High)
    );
    assert_eq!(resumed.snapshot.config.kernel["thinking"], json!("high"));
    let old = summary("ses_without_level");
    store.create(&old).await.unwrap();
    store
        .append(
            &old.id,
            &Frame {
                session: old.id.clone(),
                seq: Seq(1),
                ts: old.created_at,
                cause: None,
                event: Event::SessionUpdated {
                    summary: old.clone(),
                },
            },
        )
        .await
        .unwrap();
    let resumed = next
        .open(
            SessionSelector::ById { id: old.id },
            who(),
            OpenOptions::default(),
        )
        .await
        .unwrap();
    assert_eq!(
        next.session_thinking(&resumed.session).unwrap(),
        Some(Effort::Max)
    );
    assert_eq!(resumed.snapshot.config.kernel["thinking"], json!("max"));
    next.shutdown().await;
}

#[tokio::test(start_paused = true)]
async fn message_counts_agree_after_retry_compaction_close_and_resume() {
    let store = Arc::new(crate::journal::MemoryStore::new());
    let provider = ScriptedProvider::new(vec![
        Script::Events(vec![
            Ok(ModelEvent::TextStart { id: "lost".into() }),
            Ok(ModelEvent::TextDelta {
                id: "lost".into(),
                delta: "withdrawn".into(),
            }),
            Err(ProviderError::Server {
                status: 503,
                message: "retry".into(),
            }),
        ]),
        Script::Events(text(&"answer ".repeat(1000))),
        Script::Events(text("tail")),
    ]);
    let compactor = Arc::new(CutAtTail);
    let host = Host::build(
        vec![
            TestPlugin::boxed(&PROVIDER, vec![Contribution::Provider(provider)]),
            TestPlugin::boxed(
                &STORE,
                vec![
                    Contribution::Store(store.clone()),
                    Contribution::Compactor(compactor.clone()),
                ],
            ),
        ],
        HostConfig::new(env()).with_layer("cli", json!({"model": "m"})),
    )
    .await
    .unwrap();
    let mut a = opened(&host, spec("/work")).await;
    one_turn(&mut a, "first").await;
    one_turn(&mut a, "second").await;
    host.compact(&a.session, None).await.unwrap();
    while let Some(frame) = a.events.next().await {
        if matches!(frame.event, Event::TurnCompleted { .. }) {
            break;
        }
    }
    let before = host
        .session_summary(&a.session)
        .await
        .unwrap()
        .history
        .messages;
    host.shutdown().await;
    let frames = store.replay(&a.session, Seq::ZERO).await.unwrap();
    assert!(
        frames
            .iter()
            .any(|f| matches!(f.event, Event::TurnRetrying { .. }))
    );
    assert!(
        frames
            .iter()
            .any(|f| matches!(f.event, Event::Compacted { .. }))
    );
    let next = host_on(store.clone(), ScriptedProvider::new(vec![])).await;
    let resumed = next
        .open(
            SessionSelector::ById { id: a.session },
            who(),
            OpenOptions::default(),
        )
        .await
        .unwrap();
    assert_eq!(assert_counts(&next, &store, &resumed.session).await, before);
    next.shutdown().await;
}

async fn assert_counts(
    host: &Host,
    store: &crate::journal::MemoryStore,
    id: &SessionId,
) -> Option<u64> {
    let live = host.session_summary(id).await.unwrap().history.messages;
    let listed = store.list(&SessionFilter::default()).await.unwrap();
    let frames = store.replay(id, Seq::ZERO).await.unwrap();
    let replayed = session::replayed(&frames).unwrap();
    assert_eq!(
        live,
        listed
            .iter()
            .find(|s| &s.id == id)
            .unwrap()
            .history
            .messages
    );
    assert_eq!(live, replayed.summary.history.messages);
    live
}

struct CutAtTail;

#[async_trait]
impl Compactor for CutAtTail {
    async fn compact(
        &self,
        cx: CompactContext<'_>,
        _: CompactReason,
    ) -> Result<Compaction, CompactError> {
        ScriptedCompactor::cut(cx.items[2].id.as_str(), 2000, 10).map_err(Into::into)
    }
}
