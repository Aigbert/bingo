use super::*;
use std::sync::{
    Mutex,
    atomic::{AtomicUsize, Ordering},
};

#[derive(Default)]
struct CountingStore {
    inner: crate::journal::MemoryStore,
    checkpoint: Mutex<Option<Checkpoint>>,
    replays: AtomicUsize,
    resumes: AtomicUsize,
    tail_frames: AtomicUsize,
}

#[async_trait]
impl SessionStore for CountingStore {
    async fn create(&self, summary: &SessionSummary) -> Result<(), KernelError> {
        self.inner.create(summary).await
    }
    async fn append(&self, id: &SessionId, frame: &Frame) -> Result<(), KernelError> {
        self.inner.append(id, frame).await
    }
    async fn replay(&self, id: &SessionId, since: Seq) -> Result<Vec<Frame>, KernelError> {
        self.replays.fetch_add(1, Ordering::SeqCst);
        self.inner.replay(id, since).await
    }
    async fn list(&self, filter: &SessionFilter) -> Result<Vec<SessionSummary>, KernelError> {
        self.inner.list(filter).await
    }
    async fn delete(&self, id: &SessionId) -> Result<(), KernelError> {
        self.inner.delete(id).await
    }
    async fn checkpoint(&self, _: &SessionId, checkpoint: &Checkpoint) -> Result<(), KernelError> {
        *self.checkpoint.lock().unwrap() = Some(checkpoint.clone());
        Ok(())
    }
    async fn resume(&self, id: &SessionId) -> Result<Resumed, KernelError> {
        self.resumes.fetch_add(1, Ordering::SeqCst);
        let state = self
            .checkpoint
            .lock()
            .unwrap()
            .as_ref()
            .map(|c| c.state.clone());
        let tail = self
            .inner
            .replay(id, state.as_ref().map_or(Seq::ZERO, |s| s.seq))
            .await?;
        self.tail_frames.fetch_add(tail.len(), Ordering::SeqCst);
        Ok(Resumed { state, tail })
    }
}

async fn seed(store: &CountingStore) -> SessionId {
    let head = summary("ses_1");
    store.create(&head).await.unwrap();
    let mut events = vec![Event::SessionUpdated {
        summary: head.clone(),
    }];
    for n in 1..=4 {
        events.push(Event::ItemCompleted { item: serde_json::from_value(json!({
            "id": format!("i{n}"), "round": 0, "status": "completed",
            "startedAt": "1970-01-01T00:00:00Z", "body": { "kind": "assistant", "text": n.to_string() }
        })).unwrap() });
    }
    events.push(Event::Compacted {
        generation: 1,
        boundary: ItemId::from_raw("i3"),
        kept: vec![ItemId::from_raw("i1")],
        summary: ItemId::from_raw("i4"),
    });
    let mut state = SessionState::new(head.clone());
    for (n, event) in events.into_iter().enumerate() {
        let frame = Frame {
            seq: Seq(n as u64 + 1),
            ts: head.created_at,
            session: head.id.clone(),
            cause: None,
            event,
        };
        store.append(&head.id, &frame).await.unwrap();
        state.apply(&frame);
    }
    state.transcript.drop_before_cut();
    store
        .checkpoint(&head.id, &Checkpoint::new(state))
        .await
        .unwrap();
    store
        .append(
            &head.id,
            &Frame {
                seq: Seq(7),
                ts: head.created_at,
                session: head.id.clone(),
                cause: None,
                event: Event::SessionClosed {
                    reason: CloseReason::Client,
                },
            },
        )
        .await
        .unwrap();
    head.id
}

#[tokio::test]
async fn checkpoint_host_resume_reads_only_the_tail_and_clean_close_writes() {
    let store = Arc::new(CountingStore::default());
    let id = seed(&store).await;
    let resumed = store.resume(&id).await.unwrap();
    let mut folded = session::replayed(&store.inner.replay(&id, Seq::ZERO).await.unwrap()).unwrap();
    folded.transcript.drop_before_cut();
    assert_eq!(session::fold_resumed(&resumed).unwrap(), folded);
    store.resumes.store(0, Ordering::SeqCst);
    store.tail_frames.store(0, Ordering::SeqCst);
    let provider = ScriptedProvider::new(vec![]);
    let host = Host::build(
        vec![
            TestPlugin::boxed(&PROVIDER, vec![Contribution::Provider(provider)]),
            TestPlugin::boxed(&STORE, vec![Contribution::Store(store.clone())]),
        ],
        HostConfig::new(env()).with_layer("cli", json!({"model": "m"})),
    )
    .await
    .unwrap();
    let attachment = host
        .open(
            SessionSelector::ById { id: id.clone() },
            who(),
            OpenOptions {
                max_snapshot_bytes: Some(1024 * 1024),
                ..Default::default()
            },
        )
        .await
        .unwrap();
    assert_eq!(store.resumes.load(Ordering::SeqCst), 1);
    assert_eq!(store.tail_frames.load(Ordering::SeqCst), 1);
    assert_eq!(store.replays.load(Ordering::SeqCst), 0);
    assert_eq!(
        attachment
            .snapshot
            .transcript
            .items()
            .iter()
            .map(|i| i.id.as_str())
            .collect::<Vec<_>>(),
        ["i3", "i4"]
    );
    let mailbox = host.live(&id).unwrap().mailbox;
    mailbox.close(CloseReason::Client);
    mailbox.wait_closed().await;
    let checkpoint = store.checkpoint.lock().unwrap().clone().unwrap();
    assert!(checkpoint.state.closed);
    assert!(checkpoint.seq > Seq(7));
    assert_eq!(
        checkpoint
            .state
            .transcript
            .model_items()
            .map(|i| i.id.as_str())
            .collect::<Vec<_>>(),
        ["i1", "i4", "i3"]
    );
}
