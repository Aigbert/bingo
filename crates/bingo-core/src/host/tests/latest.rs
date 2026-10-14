//! Which session `--continue` means: the person's root, never a child of
//! it, however the openers in one process interleave.

use super::*;

/// `--continue` means the person's session: `Latest` prefers a root over a
/// child under it, live or in the store, though the child is newer.
#[tokio::test]
async fn latest_prefers_a_root_over_the_newer_child_under_it() {
    let store = Arc::new(crate::journal::MemoryStore::new());
    let host_a = host_on(store.clone(), ScriptedProvider::new(vec![])).await;
    let root = host_a
        .open(
            SessionSelector::Create {
                spec: spec("/work"),
            },
            who(),
            OpenOptions::default(),
        )
        .await
        .unwrap()
        .session;
    tokio::time::sleep(std::time::Duration::from_millis(5)).await;
    let child = SessionSpec {
        parent: Some(ParentLink {
            session: root.clone(),
            item: None,
        }),
        title: Some("reviewer".into()),
        ..spec("/work")
    };
    let child = host_a
        .open(
            SessionSelector::Create { spec: child },
            who(),
            OpenOptions::default(),
        )
        .await
        .unwrap()
        .session;
    let latest = SessionSelector::Latest {
        cwd: "/work".into(),
    };
    let live = host_a
        .open(latest.clone(), who(), OpenOptions::default())
        .await
        .unwrap();
    assert_eq!(live.session, root, "live: the root, not its newer child");

    let host_b = host_on(store.clone(), ScriptedProvider::new(vec![])).await;
    let stored = host_b
        .open(latest.clone(), who(), OpenOptions::default())
        .await
        .unwrap();
    assert_eq!(
        stored.session, root,
        "stored: the root, not its newer child"
    );

    // A plugin that wakes a child at start makes it live before its root.
    let host_c = host_on(store, ScriptedProvider::new(vec![])).await;
    host_c
        .open(
            SessionSelector::ById { id: child },
            who(),
            OpenOptions::default(),
        )
        .await
        .unwrap();
    let woken = host_c
        .open(latest, who(), OpenOptions::default())
        .await
        .unwrap();
    assert_eq!(woken.session, root, "a live child does not hide its root");
}

/// A store that holds each session for one taker, as the journal on disk
/// does, and gives way while it reads one back so two openers can overlap.
struct HeldStore {
    inner: Arc<crate::journal::MemoryStore>,
    held: std::sync::Mutex<std::collections::BTreeSet<SessionId>>,
}

#[async_trait::async_trait]
impl SessionStore for HeldStore {
    async fn create(&self, summary: &SessionSummary) -> Result<(), KernelError> {
        self.inner.create(summary).await
    }

    async fn append(&self, session: &SessionId, frame: &Frame) -> Result<(), KernelError> {
        self.inner.append(session, frame).await
    }

    async fn replay(&self, session: &SessionId, since: Seq) -> Result<Vec<Frame>, KernelError> {
        tokio::task::yield_now().await;
        self.inner.replay(session, since).await
    }

    async fn list(&self, filter: &SessionFilter) -> Result<Vec<SessionSummary>, KernelError> {
        self.inner.list(filter).await
    }

    async fn delete(&self, session: &SessionId) -> Result<(), KernelError> {
        self.inner.delete(session).await
    }

    async fn acquire(&self, session: &SessionId) -> Result<(), KernelError> {
        if self.held.lock().unwrap().insert(session.clone()) {
            Ok(())
        } else {
            Err(KernelError::new(ErrorCode::SessionLocked, "held"))
        }
    }

    async fn release(&self, session: &SessionId) -> Result<(), KernelError> {
        self.held.lock().unwrap().remove(session);
        Ok(())
    }
}

/// A plugin that reads the root at start races `--continue` for it in the
/// same process: both must end on the root, not on a child of it.
#[tokio::test]
async fn two_openers_in_one_process_meet_on_the_same_stored_root() {
    let memory = Arc::new(crate::journal::MemoryStore::new());
    let host_a = host_on(memory.clone(), ScriptedProvider::new(vec![])).await;
    let root = host_a
        .open(
            SessionSelector::Create {
                spec: spec("/work"),
            },
            who(),
            OpenOptions::default(),
        )
        .await
        .unwrap()
        .session;
    let child = SessionSpec {
        parent: Some(ParentLink {
            session: root.clone(),
            item: None,
        }),
        ..spec("/work")
    };
    host_a
        .open(
            SessionSelector::Create { spec: child },
            who(),
            OpenOptions::default(),
        )
        .await
        .unwrap();

    let held = Arc::new(HeldStore {
        inner: memory,
        held: std::sync::Mutex::new(Default::default()),
    });
    let host_b = host_on(held, ScriptedProvider::new(vec![])).await;
    let by_id = SessionSelector::ById { id: root.clone() };
    let latest = SessionSelector::Latest {
        cwd: "/work".into(),
    };
    let (read, continued) = tokio::join!(
        host_b.open(by_id, who(), OpenOptions::default()),
        host_b.open(latest, who(), OpenOptions::default()),
    );
    assert_eq!(read.unwrap().session, root);
    assert_eq!(continued.unwrap().session, root, "--continue is the root");
}
