use super::*;
#[tokio::test]
async fn fork_keeps_ids_configuration_and_the_cut_but_no_children() {
    let store = Arc::new(crate::journal::MemoryStore::new());
    let provider = ScriptedProvider::new(vec![
        Script::Events(text("one")),
        Script::Events(text("two")),
    ]);
    let host = host_on(store.clone(), provider).await;
    let mut source = host
        .open(
            SessionSelector::Create {
                spec: spec("/work"),
            },
            who(),
            OpenOptions::default(),
        )
        .await
        .unwrap();
    one_turn(&mut source, "first").await;
    let first = host.session_state(&source.session).await.unwrap();
    one_turn(&mut source, "second").await;
    let child = host
        .open(
            SessionSelector::Create {
                spec: SessionSpec {
                    cwd: "/work".into(),
                    driver: Driver::Log,
                    parent: Some(ParentLink {
                        session: source.session.clone(),
                        item: None,
                    }),
                    ..Default::default()
                },
            },
            who(),
            OpenOptions::default(),
        )
        .await
        .unwrap();
    assert_eq!(
        child.snapshot.summary.parent.as_ref().unwrap().session,
        source.session
    );
    let cut = first.last_turn.as_ref().unwrap().id.clone();
    let id = host.fork(&source.session, Some(cut.clone())).await.unwrap();
    let fork = host
        .open(
            SessionSelector::ById { id: id.clone() },
            who(),
            OpenOptions::default(),
        )
        .await
        .unwrap();
    assert_eq!(fork.snapshot.transcript.items(), first.transcript.items());
    assert_eq!(
        fork.snapshot.summary.history.first_ask.as_deref(),
        Some("first")
    );
    assert_eq!(fork.snapshot.summary.model, first.summary.model);
    assert!(
        host.sessions(SessionFilter {
            parent: Some(id.clone()),
            ..Default::default()
        })
        .await
        .unwrap()
        .is_empty()
    );
    assert_eq!(
        host.sessions(SessionFilter {
            parent: Some(source.session.clone()),
            ..Default::default()
        })
        .await
        .unwrap()
        .len(),
        1
    );
    assert!(fork.snapshot.summary.parent.is_none());
    assert!(fork.snapshot.summary.key.is_none());
    assert_eq!(
        fork.snapshot.summary.history.forked_from,
        Some(ForkLink {
            session: source.session.clone(),
            turn: Some(cut)
        })
    );
    assert!(
        store
            .replay(&id, Seq::ZERO)
            .await
            .unwrap()
            .iter()
            .all(|f| f.session == id)
    );
    assert_eq!(
        host.fork(&source.session, Some(TurnId::from_raw("unknown")))
            .await
            .unwrap_err()
            .code,
        ErrorCode::InvalidInput
    );
    let whole = host.fork(&source.session, None).await.unwrap();
    assert_ne!(whole, id);
    host.shutdown().await;
}
#[tokio::test]
async fn fork_refuses_a_running_turn() {
    let store = Arc::new(crate::journal::MemoryStore::new());
    let provider = ScriptedProvider::new(vec![Script::Hang(vec![])]);
    let host = host_on(store, provider).await;
    let mut source = host
        .open(
            SessionSelector::Create {
                spec: spec("/work"),
            },
            who(),
            OpenOptions::default(),
        )
        .await
        .unwrap();
    source.handle.submit(
        IntentId::mint(),
        Input::text("wait", Origin::surface("test")),
    );
    while let Some(frame) = source.events.next().await {
        if matches!(frame.event, Event::TurnStarted { .. }) {
            break;
        }
    }
    assert_eq!(
        host.fork(&source.session, None).await.unwrap_err().code,
        ErrorCode::NotReady
    );
    assert_eq!(
        host.delete(&source.session).await.unwrap_err().code,
        ErrorCode::NotReady
    );
    assert!(
        host.read(&source.session).await.unwrap().busy(),
        "delete must not end the turn"
    );
    host.shutdown().await;
}

/// A store that refuses one step for every session but the source.
struct Refusing {
    inner: crate::journal::MemoryStore,
    source: std::sync::Mutex<Option<SessionId>>,
    step: &'static str,
}

impl Refusing {
    fn refuses(&self, id: &SessionId, step: &str) -> Result<(), KernelError> {
        let source = self.source.lock().unwrap();
        if step == self.step && source.as_ref().is_some_and(|source| source != id) {
            return Err(KernelError::new(ErrorCode::Internal, step));
        }
        Ok(())
    }
}

#[async_trait]
impl SessionStore for Refusing {
    async fn create(&self, summary: &SessionSummary) -> Result<(), KernelError> {
        self.inner.create(summary).await
    }
    async fn append(&self, id: &SessionId, frame: &Frame) -> Result<(), KernelError> {
        self.refuses(id, "append")?;
        self.inner.append(id, frame).await
    }
    async fn replay(&self, id: &SessionId, since: Seq) -> Result<Vec<Frame>, KernelError> {
        self.inner.replay(id, since).await
    }
    async fn list(&self, filter: &SessionFilter) -> Result<Vec<SessionSummary>, KernelError> {
        self.inner.list(filter).await
    }
    async fn summary(&self, id: &SessionId) -> Result<SessionSummary, KernelError> {
        self.inner.summary(id).await
    }
    async fn delete(&self, id: &SessionId) -> Result<(), KernelError> {
        self.inner.delete(id).await
    }
    async fn release(&self, id: &SessionId) -> Result<(), KernelError> {
        self.refuses(id, "release")?;
        self.inner.release(id).await
    }
}

/// Forks a one-turn session over a store that refuses `step` for the copy,
/// and answers the fork's outcome with the sessions left in the store.
async fn fork_refusing(step: &'static str) -> (Result<SessionId, KernelError>, usize) {
    let store = Arc::new(Refusing {
        inner: crate::journal::MemoryStore::new(),
        source: std::sync::Mutex::new(None),
        step,
    });
    let provider = ScriptedProvider::new(vec![Script::Events(text("one"))]);
    let host = host_on(store.clone(), provider).await;
    let mut source = host
        .open(
            SessionSelector::Create {
                spec: spec("/work"),
            },
            who(),
            OpenOptions::default(),
        )
        .await
        .unwrap();
    one_turn(&mut source, "first").await;
    *store.source.lock().unwrap() = Some(source.session.clone());
    let forked = host.fork(&source.session, None).await;
    let left = store.list(&SessionFilter::default()).await.unwrap().len();
    host.shutdown().await;
    (forked, left)
}

#[tokio::test]
async fn a_finished_fork_survives_a_failed_release() {
    let (forked, left) = fork_refusing("release").await;
    assert!(forked.is_ok(), "{forked:?}");
    assert_eq!(left, 2, "the copy is complete and stays");
}

#[tokio::test]
async fn an_unfinished_fork_is_gone_before_the_error_returns() {
    let (forked, left) = fork_refusing("append").await;
    assert_eq!(forked.unwrap_err().message, "append");
    assert_eq!(left, 1, "only the source remains");
}
