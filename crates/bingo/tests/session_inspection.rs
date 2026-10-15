#![allow(clippy::unwrap_used, clippy::expect_used)]
use async_trait::async_trait;
use bingo_core::{Host, HostConfig};
use bingo_sdk::*;
use bingo_store_jsonl::JsonlStore;
use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};

struct Fixture {
    _home: tempfile::TempDir,
    store: Arc<JsonlStore>,
    host: Arc<Host>,
    opened: Arc<AtomicUsize>,
}
struct PluginFixture(Arc<JsonlStore>, Arc<AtomicUsize>);
static MANIFEST: PluginManifest = PluginManifest {
    id: "inspection",
    version: "1",
    uses: &[],
    provides: &[],
    requires: &[],
    config: None,
};
#[async_trait]
impl Plugin for PluginFixture {
    fn manifest(&self) -> &'static PluginManifest {
        &MANIFEST
    }
    fn register(&self, registrar: &mut Registrar) -> Result<(), PluginError> {
        registrar.add(Contribution::Store(self.0.clone()));
        registrar.add(Contribution::Hook(Arc::new(Opened(self.1.clone()))));
        Ok(())
    }
}
struct Opened(Arc<AtomicUsize>);
#[async_trait]
impl Hook for Opened {
    fn id(&self) -> &str {
        "open-counter"
    }
    fn matcher(&self) -> HookMatcher {
        HookMatcher {
            points: vec![HookPoint::Session],
            tool: None,
        }
    }
    async fn on_session(&self, phase: Phase, _: &HookContext) {
        if phase == Phase::Start {
            self.0.fetch_add(1, Ordering::SeqCst);
        }
    }
}
impl Fixture {
    async fn new() -> Self {
        let home = tempfile::tempdir().unwrap();
        let store = Arc::new(JsonlStore::new(home.path().join("sessions")));
        let opened = Arc::new(AtomicUsize::new(0));
        let host = Host::build(
            vec![Box::new(PluginFixture(store.clone(), opened.clone()))],
            HostConfig::new(Env::rooted(home.path())),
        )
        .await
        .unwrap();
        Self {
            _home: home,
            store,
            host,
            opened,
        }
    }
    async fn seed(&self, name: &str, parent: Option<SessionId>) -> SessionId {
        let mut summary: SessionSummary = serde_json::from_value(serde_json::json!({"id":name,"cwd":"/work","createdAt":"2026-10-15T00:00:00Z","updatedAt":"2026-10-15T00:00:00Z"})).unwrap();
        summary.parent = parent.map(|session| ParentLink {
            session,
            item: None,
        });
        self.store.create(&summary).await.unwrap();
        let id = summary.id.clone();
        for event in [
            Event::SessionUpdated { summary },
            Event::TurnCompleted {
                turn: TurnId::from_raw("complete"),
                status: TurnStatus::Completed,
                usage: Usage::default(),
            },
        ] {
            self.append(&id, event).await;
        }
        id
    }
    async fn append(&self, id: &SessionId, event: Event) {
        let frames = self.store.replay(id, Seq::ZERO).await.unwrap();
        self.store
            .append(
                id,
                &Frame {
                    session: id.clone(),
                    seq: Seq(frames.len() as u64 + 1),
                    ts: jiff::Timestamp::now(),
                    cause: None,
                    event,
                },
            )
            .await
            .unwrap();
    }
    fn bytes(&self, id: &SessionId) -> Vec<u8> {
        std::fs::read(self.store.root().join(id.as_str()).join("journal.jsonl")).unwrap()
    }
}

#[tokio::test]
async fn stored_preview_fork_and_rename_never_revive_the_source() {
    let f = Fixture::new().await;
    let id = f.seed("source", None).await;
    let before = f.bytes(&id);
    let summary = f.store.summary(&id).await.unwrap();
    assert_eq!(
        f.host.read(&id).await.unwrap().summary.updated_at,
        summary.updated_at
    );
    assert_eq!(f.bytes(&id), before);
    assert!(!f.store.root().join(id.as_str()).join(".lock").exists());
    let fork = f.host.fork(&id, None).await.unwrap();
    assert_eq!(f.bytes(&id), before);
    assert_eq!(
        f.store.summary(&id).await.unwrap().updated_at,
        summary.updated_at
    );
    assert!(!f.store.locked(&id).await.unwrap());
    assert_eq!(
        f.store
            .summary(&fork)
            .await
            .unwrap()
            .history
            .forked_from
            .unwrap()
            .turn
            .unwrap()
            .as_str(),
        "complete"
    );
    f.host
        .reconfigure(&id, SessionChange::Title("changed".into()))
        .await
        .unwrap();
    assert_eq!(
        f.store.summary(&id).await.unwrap().updated_at,
        summary.updated_at
    );
    assert_eq!(f.store.replay(&id, Seq::ZERO).await.unwrap().len(), 3);
    assert_eq!(
        f.store.summary(&id).await.unwrap().title.as_deref(),
        Some("changed")
    );
    f.host
        .reconfigure(&id, SessionChange::Title(" ".into()))
        .await
        .unwrap();
    assert_eq!(f.store.summary(&id).await.unwrap().title, None);
    assert!(!f.store.locked(&id).await.unwrap());
    assert_eq!(f.opened.load(Ordering::SeqCst), 0);
    f.host.shutdown().await;
}

#[tokio::test]
async fn locked_parent_preserves_every_child_and_releases_preflight_claims() {
    let f = Fixture::new().await;
    let parent = f.seed("parent", None).await;
    let child = f.seed("child", Some(parent.clone())).await;
    let owner = JsonlStore::new(f.store.root());
    owner.acquire(&parent).await.unwrap();
    let parent_bytes = f.bytes(&parent);
    let child_bytes = f.bytes(&child);
    assert_eq!(
        f.host.delete(&parent).await.unwrap_err().code,
        ErrorCode::SessionLocked
    );
    assert_eq!(f.bytes(&parent), parent_bytes);
    assert_eq!(f.bytes(&child), child_bytes);
    assert!(!f.store.locked(&child).await.unwrap());
    assert!(f.store.locked(&parent).await.unwrap());
    assert_eq!(
        f.host
            .reconfigure(&parent, SessionChange::Title("stolen".into()))
            .await
            .unwrap_err()
            .code,
        ErrorCode::SessionLocked
    );
    assert_eq!(f.bytes(&parent), parent_bytes);
    owner.release(&parent).await.unwrap();
    f.host.delete(&parent).await.unwrap();
    assert!(
        f.store
            .list(&SessionFilter::default())
            .await
            .unwrap()
            .is_empty()
    );
    f.host.shutdown().await;
}

#[tokio::test]
async fn foreign_mid_turn_fork_refuses_and_a_crashed_tail_is_excluded() {
    let f = Fixture::new().await;
    let id = f.seed("source", None).await;
    let owner = JsonlStore::new(f.store.root());
    owner.acquire(&id).await.unwrap();
    // A foreign idle writer is safe: copying a completed prefix takes no claim.
    f.host.fork(&id, None).await.unwrap();
    f.append(
        &id,
        Event::TurnStarted {
            turn: TurnId::from_raw("unfinished"),
            inputs: vec![],
            origin: TurnOrigin::Submit,
        },
    )
    .await;
    let before = f.bytes(&id);
    assert!(f.host.read(&id).await.unwrap().busy());
    let error = f.host.fork(&id, None).await.unwrap_err();
    assert_eq!(error.code, ErrorCode::SessionLocked);
    assert!(error.message.contains("running elsewhere"));
    owner.release(&id).await.unwrap();
    let fork = f.host.fork(&id, None).await.unwrap();
    let copied = f.store.replay(&fork, Seq::ZERO).await.unwrap();
    assert!(
        !copied
            .iter()
            .any(|frame| matches!(frame.event, Event::TurnStarted { .. }))
    );
    assert_eq!(f.bytes(&id), before);
    assert!(!f.store.locked(&id).await.unwrap());
    assert_eq!(f.opened.load(Ordering::SeqCst), 0);
    f.host.shutdown().await;
}
