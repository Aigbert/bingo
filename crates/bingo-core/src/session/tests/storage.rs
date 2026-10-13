use super::*;
use crate::journal::MemoryStore;
use std::sync::atomic::{AtomicUsize, Ordering};

#[derive(Default)]
struct FailingStore {
    inner: MemoryStore,
    appends: AtomicUsize,
    fail_at: AtomicUsize,
}

#[async_trait::async_trait]
impl SessionStore for FailingStore {
    async fn create(&self, summary: &SessionSummary) -> Result<(), KernelError> {
        self.inner.create(summary).await
    }
    async fn append(&self, id: &SessionId, frame: &Frame) -> Result<(), KernelError> {
        let n = self.appends.fetch_add(1, Ordering::SeqCst) + 1;
        if n == self.fail_at.load(Ordering::SeqCst) {
            return Err(KernelError::new(ErrorCode::Storage, "disk full"));
        }
        self.inner.append(id, frame).await
    }
    async fn replay(&self, id: &SessionId, since: Seq) -> Result<Vec<Frame>, KernelError> {
        self.inner.replay(id, since).await
    }
    async fn list(&self, filter: &SessionFilter) -> Result<Vec<SessionSummary>, KernelError> {
        self.inner.list(filter).await
    }
    async fn delete(&self, id: &SessionId) -> Result<(), KernelError> {
        self.inner.delete(id).await
    }
}

async fn failing_actor() -> (Actor, Arc<FailingStore>) {
    let store = Arc::new(FailingStore::default());
    let head = summary("ses_1");
    store.create(&head).await.unwrap();
    let (tx, rx) = mpsc::unbounded_channel();
    let (done, finished) = watch::channel(false);
    let mailbox = Mailbox::new(head.id.clone(), tx, finished);
    let cfg = Arc::new(config(
        ScriptedProvider::new(vec![Script::Establishing]),
        vec![],
        Arc::new(NoHost),
    ));
    let actor = Actor {
        id: head.id.clone(),
        mailbox: mailbox.clone(),
        rx,
        state: SessionState::new(head),
        journal: vec![],
        seq: Seq::ZERO,
        store: Some(store.clone()),
        config: cfg,
        subscribers: Subscribers::default(),
        running: None,
        queue: Queue::default(),
        commands: Commands::new(Services::none(), mailbox, "/tmp".into()),
        tracker: TaskTracker::new(),
        observed: None,
        done,
        pending: HashMap::new(),
        closing: None,
        progress_n: 0,
    };
    (actor, store)
}

#[tokio::test]
async fn storage_failure_never_publishes_or_observes_the_failed_frame() {
    let (mut actor, store) = failing_actor().await;
    actor.open().await.unwrap().await;
    let events = actor.subscribe(actor.seq);
    let (observed, mut observations) = mpsc::unbounded_channel();
    actor.observed = Some(observed);
    let cancel = CancellationToken::new();
    actor.running = Some(Running::spawn(
        &actor,
        TurnId::mint(),
        cancel.clone(),
        TurnKind::Respond,
    ));
    let queued = IntentId::mint();
    actor.queue.push(
        queued.clone(),
        Input::text("queued", Origin::surface("test")),
    );
    store
        .fail_at
        .store(store.appends.load(Ordering::SeqCst) + 1, Ordering::SeqCst);
    let event = Event::Extension {
        plugin: "test".into(),
        kind: "failed".into(),
        payload: json!(1),
    };
    let _ = actor.publish(event, None).await;
    assert!(!actor.state.extensions.contains_key("test"));
    assert!(
        !actor
            .journal
            .iter()
            .any(|f| matches!(f.event, Event::Extension { .. }))
    );
    assert!(cancel.is_cancelled());
    assert!(actor.state.closed);
    assert!(actor.running.is_none());
    let frames: Vec<_> = events.collect().await;
    assert!(
        frames
            .iter()
            .any(|f| matches!(&f.event, Event::Notice { text, .. } if text.contains("disk full")))
    );
    assert!(frames.iter().any(|f| matches!(&f.event, Event::IntentAck { intent, outcome: IntentOutcome::Rejected { error } } if intent == &queued && error.code == ErrorCode::Storage)));
    assert!(
        matches!(&frames.last().unwrap().event, Event::SessionClosed { reason: CloseReason::Error { message } } if message.contains("disk full"))
    );
    assert!(
        !frames
            .iter()
            .any(|f| matches!(f.event, Event::Extension { .. }))
    );
    while let Ok((frame, _)) = observations.try_recv() {
        assert!(!matches!(frame.event, Event::Extension { .. }));
    }
}

#[tokio::test]
async fn storage_failure_stops_the_actor_mailbox() {
    let store = Arc::new(FailingStore::default());
    store.create(&summary("ses_1")).await.unwrap();
    let mailbox = spawn(
        summary("ses_1"),
        Some(store.clone()),
        Services::none(),
        |_| {
            Arc::new(config(
                ScriptedProvider::new(vec![]),
                vec![],
                Arc::new(NoHost),
            ))
        },
    );
    let (_, mut events) = mailbox.attach().await.unwrap();
    store
        .fail_at
        .store(store.appends.load(Ordering::SeqCst) + 1, Ordering::SeqCst);
    mailbox.extend("test".into(), "failed".into(), json!(1));
    let frame = events.next().await.unwrap();
    assert!(matches!(frame.event, Event::Notice { .. }));
    mailbox.wait_closed().await;
    assert_eq!(
        mailbox.summary().await.unwrap_err().code,
        ErrorCode::SessionClosed
    );
}

#[tokio::test]
async fn storage_failure_stops_opening_at_the_first_failed_append() {
    for nth in 1..=2 {
        let (mut actor, store) = failing_actor().await;
        store.fail_at.store(nth, Ordering::SeqCst);
        assert!(actor.open().await.is_err());
        assert_eq!(store.appends.load(Ordering::SeqCst), nth);
        assert_eq!(actor.journal.len(), nth - 1);
        assert!(actor.state.closed);
    }
}

#[tokio::test]
async fn storage_failure_stops_a_batch_before_spawning_a_turn() {
    let (mut actor, store) = failing_actor().await;
    actor.open().await.unwrap().await;
    let before = store.appends.load(Ordering::SeqCst);
    store.fail_at.store(before + 1, Ordering::SeqCst);
    let inputs = ["one", "two"]
        .into_iter()
        .map(|text| (IntentId::mint(), Input::text(text, Origin::surface("test"))))
        .collect();
    assert!(
        actor
            .start_turn(inputs, TurnOrigin::Submit, TurnKind::Respond)
            .await
            .is_err()
    );
    assert_eq!(store.appends.load(Ordering::SeqCst), before + 1);
    assert_eq!(actor.journal.len(), before);
    assert!(actor.state.items.is_empty());
    assert!(actor.running.is_none());
}
