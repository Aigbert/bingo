use super::*;
use std::sync::Mutex;

struct Watch(Arc<Mutex<Vec<String>>>);
#[async_trait]
impl Interceptor for Watch {
    fn id(&self) -> &str {
        "watch"
    }
    fn matcher(&self) -> LinkMatcher {
        LinkMatcher::default()
    }
    async fn observe(&self, entry: &bingo_sdk::catalogue::Entry, _: &HookContext, _: &Value) {
        self.0.lock().unwrap().push(entry.name.to_string());
    }
    async fn chain(
        &self,
        entry: &bingo_sdk::catalogue::Entry,
        _: &HookContext,
        input: Value,
        next: Next<'_, Value, Value>,
    ) -> Result<Value, KernelError> {
        if entry == bingo_sdk::catalogue::TURN_COMPLETE {
            assert_eq!(input["answer"], "answer");
            assert_eq!(next.run(input).await?, Value::Null);
            return Ok(json!("under the answer"));
        }
        next.run(input).await
    }
}

#[tokio::test]
async fn lifecycle_facts_are_ordered_and_completion_text_follows_the_answer() {
    let seen = Arc::new(Mutex::new(Vec::new()));
    let provider = ScriptedProvider::new(vec![Script::Events(text("answer"))]);
    let host = Host::build(
        vec![TestPlugin::boxed(
            &PROVIDER,
            vec![
                Contribution::Provider(provider),
                Contribution::Interceptor {
                    external: None,
                    link: Arc::new(Watch(seen.clone())),
                },
            ],
        )],
        HostConfig::new(env()).with_layer("cli", json!({"model":"m"})),
    )
    .await
    .unwrap();
    let mut attachment = host
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
    attachment.handle.submit(
        IntentId::mint(),
        Input::text("hello", Origin::surface("test")),
    );
    let mut completed = false;
    while let Some(frame) = attachment.events.next().await {
        match frame.event {
            Event::TurnCompleted { .. } => completed = true,
            Event::ItemCompleted { item } if matches!(&item.body, ItemBody::Notice { code, .. } if code == "TURN_COMPLETE") =>
            {
                assert!(completed);
                assert!(
                    matches!(item.body, ItemBody::Notice { text, .. } if text == "under the answer")
                );
                break;
            }
            _ => {}
        }
    }
    assert!(completed);
    drop(attachment);
    host.shutdown().await;
    assert_eq!(
        *seen.lock().unwrap(),
        [
            "session.start",
            "session.attach",
            "turn.start",
            "session.measure",
            "session.detach",
            "session.end"
        ]
    );
}

struct DenySetting;
#[async_trait]
impl Interceptor for DenySetting {
    fn id(&self) -> &str {
        "deny-setting"
    }
    fn matcher(&self) -> LinkMatcher {
        LinkMatcher::at(bingo_sdk::catalogue::CONFIG_SET)
    }
    async fn chain(
        &self,
        _: &bingo_sdk::catalogue::Entry,
        _: &HookContext,
        _: Value,
        _: Next<'_, Value, Value>,
    ) -> Result<Value, KernelError> {
        Err(KernelError::new(
            ErrorCode::PermissionDenied,
            "setting denied",
        ))
    }
}

#[tokio::test]
async fn a_denied_setting_changes_neither_the_session_nor_the_file() {
    let directory = tempfile::tempdir().unwrap();
    let environment = Env {
        home: directory.path().into(),
        config_dir: directory.path().join("config"),
        data_dir: directory.path().join("data"),
    };
    let path = crate::settings::user_path(&environment);
    let host = Host::build(
        vec![TestPlugin::boxed(
            &PROVIDER,
            vec![
                Contribution::Provider(ScriptedProvider::new(vec![])),
                Contribution::Interceptor {
                    external: None,
                    link: Arc::new(DenySetting),
                },
            ],
        )],
        HostConfig::new(environment).with_layer("cli", json!({"model":"m"})),
    )
    .await
    .unwrap();
    let mut client = super::commands::Client::open(&host).await;
    let session = client.state.summary.id.clone();
    let before = host.session_model(&session).await.unwrap().unwrap().id;
    let refused = host
        .reconfigure(
            &session,
            SessionChange::Model {
                provider: None,
                model: "changed".into(),
            },
        )
        .await;
    assert!(refused.is_err(), "direct reconfigure bypassed config.set");
    let (outcome, _) = client.ack("/model changed").await;
    assert!(matches!(outcome, IntentOutcome::Rejected { .. }));
    assert_eq!(
        host.session_model(&session).await.unwrap().unwrap().id,
        before
    );
    assert!(!path.exists());
    host.shutdown().await;
}

struct PausedObserver {
    entered: Arc<tokio::sync::Notify>,
    release: Arc<tokio::sync::Notify>,
}
#[async_trait]
impl Interceptor for PausedObserver {
    fn id(&self) -> &str {
        "paused-observer"
    }
    fn matcher(&self) -> LinkMatcher {
        LinkMatcher::at(bingo_sdk::catalogue::SESSION_START)
    }
    async fn observe(&self, _: &bingo_sdk::catalogue::Entry, _: &HookContext, _: &Value) {
        self.entered.notify_one();
        self.release.notified().await;
    }
}

#[tokio::test]
async fn a_passive_observer_cannot_hold_a_command_acknowledgement() {
    let entered = Arc::new(tokio::sync::Notify::new());
    let release = Arc::new(tokio::sync::Notify::new());
    let host = Host::build(
        vec![TestPlugin::boxed(
            &PROVIDER,
            vec![
                Contribution::Provider(ScriptedProvider::new(vec![])),
                Contribution::Interceptor {
                    external: None,
                    link: Arc::new(PausedObserver {
                        entered: entered.clone(),
                        release: release.clone(),
                    }),
                },
            ],
        )],
        HostConfig::new(env()).with_layer("cli", json!({"model":"m"})),
    )
    .await
    .unwrap();
    let mut client = super::commands::Client::open(&host).await;
    tokio::time::timeout(std::time::Duration::from_secs(5), entered.notified())
        .await
        .unwrap();
    let ack = tokio::time::timeout(std::time::Duration::from_secs(5), client.ack("/status")).await;
    release.notify_one();
    host.shutdown().await;
    assert!(matches!(ack.unwrap().0, IntentOutcome::Applied { .. }));
}
