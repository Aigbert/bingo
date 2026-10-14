//! Reopening preserves the choices recorded in the session.
use super::*;

/// What a session was opened with comes back with it: its extra system
/// prompt and its tool set are in its summary, so a resume gives them back.
#[tokio::test]
async fn a_resumed_session_keeps_its_system_prompt_and_tool_set() {
    let store = Arc::new(crate::journal::MemoryStore::new());
    let host_a = host_on(store.clone(), ScriptedProvider::new(vec![])).await;
    let opened = SessionSpec {
        system_extra: Some("Be brief.".into()),
        tools: Some(vec!["Echo".into()]),
        ..spec("/work")
    };
    let id = host_a
        .open(
            SessionSelector::Create { spec: opened },
            who(),
            OpenOptions::default(),
        )
        .await
        .unwrap()
        .session;

    let second = ScriptedProvider::new(vec![Script::Events(text("ok"))]);
    let plugins = vec![
        TestPlugin::boxed(&PROVIDER, vec![Contribution::Provider(second.clone())]),
        TestPlugin::boxed(&STORE, vec![Contribution::Store(store)]),
        TestPlugin::boxed(
            &TOOLS,
            vec![Contribution::Tool(Arc::new(EchoTool { read_only: true }))],
        ),
    ];
    let config = HostConfig::new(env()).with_layer("cli", json!({"model": "m"}));
    let host_b = Host::build(plugins, config).await.unwrap();
    let mut b = host_b
        .open(SessionSelector::ById { id }, who(), OpenOptions::default())
        .await
        .unwrap();
    assert_eq!(
        b.snapshot.summary.system_extra.as_deref(),
        Some("Be brief.")
    );
    assert_eq!(b.snapshot.summary.tools, Some(vec!["Echo".to_string()]));
    one_turn(&mut b, "hello").await;
    let request = &second.requests()[0];
    assert!(
        request
            .system
            .iter()
            .any(|block| block.text.contains("Be brief.")),
        "the resumed turn's system prompt carries it"
    );
    assert_eq!(
        request
            .tools
            .iter()
            .map(|t| t.name.as_str())
            .collect::<Vec<_>>(),
        ["Echo"],
        "the resumed turn is held to the tool set"
    );
}

/// A session that was moved to another model with `/model` comes back on
/// that model, and says so (user-reported: "resume 后模型展示和实际使用的模型
/// 不对"). Two halves of one fact: the spec a resume rebuilds is what the
/// session *last* was, not what it first was — the journal's first frame
/// predates the `/model` that rewrote it (ADR-0008 §4) — and the head of the
/// new segment is stamped from the choice that was actually resolved, so no
/// surface can be shown a model that is not the one answering.
#[tokio::test]
async fn a_resumed_session_comes_back_on_the_model_it_was_moved_to() {
    let store = Arc::new(crate::journal::MemoryStore::new());
    let host_a = host_on(store.clone(), ScriptedProvider::new(vec![])).await;
    let mut a = host_a
        .open(
            SessionSelector::Create {
                spec: spec("/work"),
            },
            who(),
            OpenOptions::default(),
        )
        .await
        .unwrap();
    assert_eq!(a.snapshot.summary.model.as_deref(), Some("m"));
    let id = a.session.clone();

    a.handle.submit(
        IntentId::mint(),
        Input::text("/model m2", Origin::surface("test")),
    );
    while let Some(frame) = a.events.next().await {
        a.snapshot.apply(&frame);
        if matches!(frame.event, Event::IntentAck { .. }) {
            break;
        }
    }
    assert_eq!(a.snapshot.summary.model.as_deref(), Some("m2"));

    let answers = ScriptedProvider::new(vec![Script::Events(text("back"))]);
    let host_b = host_on(store.clone(), answers.clone()).await;
    let mut b = host_b
        .open(
            SessionSelector::ById { id: id.clone() },
            who(),
            OpenOptions::default(),
        )
        .await
        .unwrap();
    assert_eq!(
        b.snapshot.summary.model.as_deref(),
        Some("m2"),
        "the resumed session says the model it was left on"
    );

    one_turn(&mut b, "hello").await;
    assert_eq!(
        answers.requests()[0].model,
        "m2",
        "and it is the model the turn asked for"
    );
}

/// The level a session was thinking at is in its own config view, and a
/// resume reads the fold — so it comes back thinking as hard as it was left
/// (user-reported: "thinking 没有记住, 现在默认貌似是 off"). The settings say
/// nothing about thinking here, which is what would have made it `off`.
#[tokio::test]
async fn a_resumed_session_thinks_as_hard_as_it_was_left() {
    let store = Arc::new(crate::journal::MemoryStore::new());
    let host_a = reasoning_host_on(store.clone(), ScriptedProvider::new(vec![])).await;
    let mut a = host_a
        .open(
            SessionSelector::Create {
                spec: spec("/work"),
            },
            who(),
            OpenOptions::default(),
        )
        .await
        .unwrap();
    let id = a.session.clone();
    a.handle.submit(
        IntentId::mint(),
        Input::text("/think high", Origin::surface("test")),
    );
    while let Some(frame) = a.events.next().await {
        a.snapshot.apply(&frame);
        if matches!(frame.event, Event::IntentAck { .. }) {
            break;
        }
    }
    assert_eq!(a.snapshot.config.kernel["thinking"], json!("high"));

    let answers = ScriptedProvider::new(vec![Script::Events(text("back"))]);
    let host_b = reasoning_host_on(store.clone(), answers.clone()).await;
    let mut b = host_b
        .open(SessionSelector::ById { id }, who(), OpenOptions::default())
        .await
        .unwrap();
    assert_eq!(
        b.snapshot.config.kernel["thinking"],
        json!("high"),
        "the resumed session says the level it was left at"
    );
    one_turn(&mut b, "hello").await;
    assert_eq!(
        answers.requests()[0].reasoning,
        Some(Effort::High),
        "and it is the level the turn asked for"
    );
}

/// [`host_on`] with a model that declares reasoning, and no thinking level
/// in the settings — what would make a resumed session `off`.
async fn reasoning_host_on(
    store: Arc<crate::journal::MemoryStore>,
    provider: Arc<ScriptedProvider>,
) -> Arc<Host> {
    let plugins = vec![
        TestPlugin::boxed(&PROVIDER, vec![Contribution::Provider(provider)]),
        TestPlugin::boxed(&STORE, vec![Contribution::Store(store)]),
    ];
    let config = HostConfig::new(env()).with_layer(
        "cli",
        json!({"model": "m", "models": {"scripted/m": {"reasoning": true}}}),
    );
    Host::build(plugins, config).await.unwrap()
}

/// A name outlives the process that gave it: `/rename` goes onto the summary,
/// the summary is the journal's, and a resume reads the journal's fold — so
/// the session comes back called what it was called.
#[tokio::test]
async fn a_renamed_session_comes_back_under_its_name() {
    let store = Arc::new(crate::journal::MemoryStore::new());
    let host_a = host_on(store.clone(), ScriptedProvider::new(vec![])).await;
    let mut a = host_a
        .open(
            SessionSelector::Create {
                spec: spec("/work"),
            },
            who(),
            OpenOptions::default(),
        )
        .await
        .unwrap();
    let id = a.session.clone();

    a.handle.submit(
        IntentId::mint(),
        Input::text("/rename the release", Origin::surface("test")),
    );
    while let Some(frame) = a.events.next().await {
        a.snapshot.apply(&frame);
        if matches!(frame.event, Event::IntentAck { .. }) {
            break;
        }
    }
    assert_eq!(a.snapshot.summary.title.as_deref(), Some("the release"));

    let host_b = host_on(store.clone(), ScriptedProvider::new(vec![])).await;
    let b = host_b
        .open(
            SessionSelector::ById { id: id.clone() },
            who(),
            OpenOptions::default(),
        )
        .await
        .unwrap();
    assert_eq!(b.snapshot.summary.title.as_deref(), Some("the release"));
    assert_eq!(
        host_b
            .sessions(SessionFilter::default())
            .await
            .unwrap()
            .iter()
            .find(|s| s.id == id)
            .and_then(|s| s.title.clone())
            .as_deref(),
        Some("the release"),
        "and every list that names it says the same"
    );
}
