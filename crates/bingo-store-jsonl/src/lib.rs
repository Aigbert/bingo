//! The session store on disk: one directory per session holding a JSONL
//! journal, a sidecar lock and a derived summary (ADR-0005).
//!
//! The journal is the session; the summary exists so `list` never reads a
//! journal body, and the lock is the only claim of ownership. The kernel says
//! when to take and give back that claim — `create` does not lock by itself.

mod blobs;
mod checkpoint;
pub mod gc;
mod inspection;
pub mod journal;
pub mod layout;
pub mod lock;
mod reader;
mod record;
pub mod summary;

use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, OnceLock};

use async_trait::async_trait;
use bingo_sdk::{
    Contribution, ErrorCode, Event, Frame, HostHandle, KernelError, Plugin, PluginError,
    PluginManifest, Registrar, Seq, SessionFilter, SessionId, SessionStore, SessionSummary,
};
use jiff::Timestamp;

use crate::gc::Gc;
use crate::lock::Locks;

/// The disk failing is the store's fault, never the kernel's.
pub(crate) fn storage(message: impl Into<String>) -> KernelError {
    KernelError::new(ErrorCode::Storage, message)
}

/// `<data_dir>/sessions`, one directory per session.
#[derive(Clone, Debug)]
pub struct JsonlStore {
    root: PathBuf,
    locks: Arc<Locks>,
    writer: Arc<Mutex<journal::Writer>>,
}

impl JsonlStore {
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self {
            root: root.into(),
            locks: Arc::new(Locks::default()),
            writer: Arc::new(Mutex::new(journal::Writer::default())),
        }
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    /// The directory of a session that exists. A directory without a journal
    /// is not a session, whatever else it holds.
    fn existing(&self, session: &SessionId) -> Result<PathBuf, KernelError> {
        let dir = layout::session_dir(&self.root, session);
        if layout::journal(&dir).is_file() {
            Ok(dir)
        } else {
            Err(KernelError::new(
                ErrorCode::SessionNotFound,
                format!("no session {session}"),
            ))
        }
    }
}

impl JsonlStore {
    fn append_frame(&self, session: &SessionId, frame: &Frame) -> Result<(), KernelError> {
        let dir = self.existing(session)?;
        self.writer
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .append(&dir, frame)?;
        let derived = match &frame.event {
            Event::SessionUpdated { summary } => summary::write(&dir, summary),
            // A message is worth no frame of its own, so the derived file — not
            // the journal — is what stays fresh between one head and the next.
            Event::ItemCompleted { item } if item.body.is_message() => {
                summary::count_message(&dir, bingo_sdk::first_ask(item))
            }
            _ => Ok(()),
        };
        if let Err(error) = derived {
            tracing::warn!(%session, %error, "journal appended but summary update failed");
        }
        Ok(())
    }
}

#[async_trait]
impl SessionStore for JsonlStore {
    async fn create(&self, summary: &SessionSummary) -> Result<(), KernelError> {
        let dir = layout::session_dir(&self.root, &summary.id);
        let summary = summary.clone();
        off_runtime(move || {
            journal::create(&dir, &summary.id)?;
            summary::write(&dir, &summary)
        })
        .await
    }

    async fn append(&self, session: &SessionId, frame: &Frame) -> Result<(), KernelError> {
        if !frame.event.is_durable() {
            return Ok(());
        }
        let store = self.clone();
        let session = session.clone();
        let frame = frame.clone();
        off_runtime(move || store.append_frame(&session, &frame)).await
    }

    async fn replay(&self, session: &SessionId, since: Seq) -> Result<Vec<Frame>, KernelError> {
        let store = self.clone();
        let session = session.clone();
        off_runtime(move || journal::replay(&store.existing(&session)?, since)).await
    }

    async fn checkpoint(
        &self,
        session: &SessionId,
        checkpoint: &bingo_sdk::Checkpoint,
    ) -> Result<(), KernelError> {
        let store = self.clone();
        let session = session.clone();
        let checkpoint = checkpoint.clone();
        off_runtime(move || {
            let mut writer = store.writer.lock().unwrap_or_else(|e| e.into_inner());
            let dir = store.existing(&session)?;
            writer.prepare(&dir)?;
            checkpoint::write(&dir, &session, &checkpoint)
        })
        .await
    }

    async fn resume(&self, session: &SessionId) -> Result<bingo_sdk::Resumed, KernelError> {
        let store = self.clone();
        let session = session.clone();
        off_runtime(move || {
            let mut writer = store.writer.lock().unwrap_or_else(|e| e.into_inner());
            let dir = store.existing(&session)?;
            let resumed = checkpoint::resume(&dir, &session)?;
            if resumed.state.is_some() {
                writer.resumed(&dir);
            }
            Ok(resumed)
        })
        .await
    }

    async fn list(&self, filter: &SessionFilter) -> Result<Vec<SessionSummary>, KernelError> {
        let root = self.root.clone();
        let filter = filter.clone();
        off_runtime(move || summary::list(&root, &filter)).await
    }

    async fn summary(&self, session: &SessionId) -> Result<SessionSummary, KernelError> {
        let store = self.clone();
        let session = session.clone();
        off_runtime(move || inspection::summary(&store, &session)).await
    }

    async fn rename(&self, session: &SessionId, title: Option<&str>) -> Result<(), KernelError> {
        let store = self.clone();
        let session = session.clone();
        let title = title.map(str::to_owned);
        off_runtime(move || inspection::rename(&store, &session, title)).await
    }

    async fn locked(&self, session: &SessionId) -> Result<bool, KernelError> {
        let store = self.clone();
        let session = session.clone();
        off_runtime(move || lock::held(&layout::lock(&store.existing(&session)?))).await
    }

    async fn delete(&self, session: &SessionId) -> Result<(), KernelError> {
        let store = self.clone();
        let session = session.clone();
        off_runtime(move || {
            let dir = store.existing(&session)?;
            let mut writer = store.writer.lock().unwrap_or_else(|e| e.into_inner());
            writer.forget(&dir);
            store.locks.remove(&session, &layout::lock(&dir), || {
                std::fs::remove_dir_all(&dir)
                    .map_err(|e| storage(format!("remove {}: {e}", dir.display())))
            })
        })
        .await
    }

    /// Take the session for this process. The kernel calls this on create and
    /// on resume: creating a directory claims nothing by itself.
    async fn acquire(&self, session: &SessionId) -> Result<(), KernelError> {
        let store = self.clone();
        let session = session.clone();
        off_runtime(move || {
            let dir = store.existing(&session)?;
            store.locks.acquire(&session, &layout::lock(&dir))
        })
        .await
    }

    async fn release(&self, session: &SessionId) -> Result<(), KernelError> {
        let store = self.clone();
        let session = session.clone();
        off_runtime(move || {
            let mut writer = store.writer.lock().unwrap_or_else(|e| e.into_inner());
            writer.forget(&layout::session_dir(store.root(), &session));
            store.locks.release(&session);
            Ok(())
        })
        .await
    }
}

static MANIFEST: PluginManifest = PluginManifest {
    id: "bingo.store.jsonl",
    version: env!("CARGO_PKG_VERSION"),
    uses: &[],
    provides: &["store:jsonl", "service:bingo.sessions.reader"],
    requires: &[],
    config: None,
};

/// Registers the store under `<data_dir>/sessions` and collects what it may
/// when the host starts.
#[derive(Debug, Default)]
pub struct JsonlStorePlugin {
    /// `register` learns the root from the registrar; `start` gets no env, so
    /// the store it built is kept here rather than derived a second time.
    store: OnceLock<Arc<JsonlStore>>,
}

#[async_trait]
impl Plugin for JsonlStorePlugin {
    fn manifest(&self) -> &'static PluginManifest {
        &MANIFEST
    }

    fn register(&self, registrar: &mut Registrar) -> Result<(), PluginError> {
        let store = Arc::new(JsonlStore::new(registrar.env().data_dir.join("sessions")));
        // One host registers a plugin once; a second attempt keeps the first.
        let _ = self.store.set(Arc::clone(&store));
        registrar.add(Contribution::Store(store.clone()));
        registrar.add(Contribution::service(
            &bingo_sdk::store::reader::KEY,
            store as Arc<dyn bingo_sdk::store::reader::SessionReader>,
            None,
        ));
        Ok(())
    }

    async fn start(&self, _host: HostHandle) -> Result<(), PluginError> {
        let Some(store) = self.store.get() else {
            return Ok(());
        };
        // Collection that cannot run is not a reason to refuse to start: the
        // sessions are still readable, there are only more of them.
        let root = store.root().to_path_buf();
        match off_runtime(move || Gc::daily(Timestamp::now()).run(&root)).await {
            Ok(removed) if !removed.is_empty() => {
                tracing::info!(count = removed.len(), sessions = ?removed, "collected old sessions");
            }
            Ok(_) => {}
            Err(e) => tracing::warn!(error = %e, "session collection failed"),
        }
        Ok(())
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use bingo_sdk::{CloseReason, Level, ParentLink, Usage};

    /// The session the fixtures under `fixtures/` were written for.
    pub(crate) fn session() -> SessionId {
        SessionId::from_raw("ses_01JFIXTURE000000000000000")
    }

    pub(crate) fn stamp() -> Timestamp {
        Timestamp::from_second(1_700_000_000).expect("a timestamp")
    }

    pub(crate) fn summary() -> SessionSummary {
        SessionSummary {
            choice: bingo_sdk::SessionModel {
                model: Some("claude-fable-5".into()),
                model_scope: None,
                effort: None,
            },
            history: bingo_sdk::SessionHistory {
                first_ask: None,
                forked_from: None,
                messages: Some(0),
            },
            tools: None,
            system_extra: None,
            driver: Default::default(),
            id: session(),
            key: None,
            title: Some("the first name".into()),
            cwd: "/work".into(),
            parent: None,
            provider: Some("anthropic".into()),
            created_at: stamp(),
            updated_at: stamp(),
            usage: Usage::default(),
            busy: false,
            asking: false,
        }
    }

    pub(crate) fn frame(seq: u64, event: Event) -> Frame {
        Frame {
            seq: Seq(seq),
            ts: stamp(),
            session: session(),
            cause: None,
            event,
        }
    }

    pub(crate) fn fixture(name: &str) -> String {
        let path = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("fixtures")
            .join(name);
        std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {}: {e}", path.display()))
    }

    pub(crate) fn root_with() -> (tempfile::TempDir, JsonlStore) {
        let root = tempfile::tempdir().expect("temp root");
        let store = JsonlStore::new(root.path().join("sessions"));
        (root, store)
    }

    /// A session as the kernel leaves one — a journal whose head is the
    /// summary — last written at `when`, which is what `list` stamps as
    /// `updated_at` and what collection measures.
    pub(crate) async fn aged_session(store: &JsonlStore, id: &str, when: Timestamp) {
        let mut summary = summary();
        summary.id = SessionId::from_raw(id);
        store.create(&summary).await.expect("create");
        let mut head = frame(
            1,
            Event::SessionUpdated {
                summary: summary.clone(),
            },
        );
        head.session = summary.id.clone();
        store.append(&summary.id, &head).await.expect("append");
        let dir = layout::session_dir(store.root(), &summary.id);
        let journal = std::fs::File::options()
            .append(true)
            .open(layout::journal(&dir))
            .expect("open the journal");
        journal
            .set_modified(std::time::SystemTime::from(when))
            .expect("set the mtime");
    }

    fn ids(listed: &[SessionSummary]) -> Vec<String> {
        listed.iter().map(|s| s.id.to_string()).collect()
    }

    #[tokio::test]
    async fn summary_failure_counting_a_message_keeps_the_journal_line() {
        let (_root, store) = root_with();
        store.create(&summary()).await.unwrap();
        let head = frame(1, Event::SessionUpdated { summary: summary() });
        store.append(&session(), &head).await.unwrap();
        let path = layout::summary(&layout::session_dir(store.root(), &session()));
        std::fs::remove_file(&path).unwrap();
        std::fs::create_dir(&path).unwrap();
        let item = bingo_sdk::Item {
            id: bingo_sdk::ItemId::mint(),
            turn: None,
            round: 0,
            status: bingo_sdk::ItemStatus::Completed,
            started_at: stamp(),
            completed_at: Some(stamp()),
            intent: None,
            meta: Default::default(),
            body: bingo_sdk::ItemBody::Assistant {
                text: "answer".into(),
            },
        };
        let message = frame(2, Event::ItemCompleted { item });
        assert!(store.append(&session(), &message).await.is_ok());
        assert_eq!(
            store.replay(&session(), Seq::ZERO).await.unwrap(),
            vec![head, message]
        );
    }

    #[tokio::test]
    async fn summary_failure_does_not_reject_a_written_journal_line() {
        let (_root, store) = root_with();
        store.create(&summary()).await.unwrap();
        let path = layout::summary(&layout::session_dir(store.root(), &session()));
        std::fs::remove_file(&path).unwrap();
        std::fs::create_dir(&path).unwrap();
        let head = frame(1, Event::SessionUpdated { summary: summary() });
        assert!(store.append(&session(), &head).await.is_ok());
        assert_eq!(
            store.replay(&session(), Seq::ZERO).await.unwrap(),
            vec![head]
        );
    }

    #[tokio::test]
    async fn a_created_session_replays_what_it_appended() {
        let (_root, store) = root_with();
        store.create(&summary()).await.expect("create");
        store
            .append(
                &session(),
                &frame(1, Event::SessionUpdated { summary: summary() }),
            )
            .await
            .expect("append");
        store
            .append(
                &session(),
                &frame(
                    2,
                    Event::SessionClosed {
                        reason: CloseReason::Client,
                    },
                ),
            )
            .await
            .expect("append");

        let frames = store.replay(&session(), Seq::ZERO).await.expect("replay");
        assert_eq!(
            frames.iter().map(|f| f.seq.0).collect::<Vec<_>>(),
            vec![1, 2]
        );
        assert_eq!(
            store
                .replay(&session(), Seq(1))
                .await
                .expect("replay")
                .len(),
            1
        );
    }

    #[tokio::test]
    async fn an_ephemeral_frame_never_reaches_the_journal() {
        let (_root, store) = root_with();
        store.create(&summary()).await.expect("create");
        let notice = frame(
            1,
            Event::Notice {
                level: Level::Info,
                code: "X".into(),
                text: "not for the journal".into(),
            },
        );
        store.append(&session(), &notice).await.expect("ignored");
        assert!(
            store
                .replay(&session(), Seq::ZERO)
                .await
                .expect("replay")
                .is_empty()
        );
    }

    #[tokio::test]
    async fn an_unknown_session_is_not_found() {
        let (_root, store) = root_with();
        let ghost = SessionId::from_raw("ses_ghost");
        let frame = frame(
            1,
            Event::CatalogChanged {
                kind: "models".into(),
            },
        );
        for code in [
            store.append(&ghost, &frame).await.expect_err("append").code,
            store
                .replay(&ghost, Seq::ZERO)
                .await
                .expect_err("replay")
                .code,
            store.delete(&ghost).await.expect_err("delete").code,
            store.acquire(&ghost).await.expect_err("acquire").code,
        ] {
            assert_eq!(code, ErrorCode::SessionNotFound);
        }
    }

    #[tokio::test]
    async fn a_second_store_on_one_root_cannot_open_a_held_session() {
        let (root, mine) = root_with();
        let theirs = JsonlStore::new(root.path().join("sessions"));
        mine.create(&summary()).await.expect("create");

        mine.acquire(&session()).await.expect("the first holder");
        let err = theirs.acquire(&session()).await.expect_err("held");
        assert_eq!(err.code, ErrorCode::SessionLocked);
        assert!(err.message.contains(session().as_str()), "{err}");

        mine.release(&session()).await.expect("release");
        theirs.acquire(&session()).await.expect("released");
    }

    #[tokio::test]
    async fn the_holder_can_delete_what_it_holds() {
        let (_root, store) = root_with();
        store.create(&summary()).await.expect("create");
        store.acquire(&session()).await.expect("acquire");
        store.delete(&session()).await.expect("delete");
        assert!(
            store
                .list(&SessionFilter::default())
                .await
                .expect("list")
                .is_empty()
        );
        assert_eq!(
            store.delete(&session()).await.expect_err("gone").code,
            ErrorCode::SessionNotFound
        );
    }

    #[tokio::test]
    async fn list_answers_most_recently_updated_first() {
        let (_root, store) = root_with();
        aged_session(&store, "ses_a", stamp()).await;
        aged_session(
            &store,
            "ses_b",
            Timestamp::from_second(stamp().as_second() + 60).expect("later"),
        )
        .await;
        aged_session(
            &store,
            "ses_c",
            Timestamp::from_second(stamp().as_second() - 60).expect("earlier"),
        )
        .await;

        let listed = store.list(&SessionFilter::default()).await.expect("list");
        assert_eq!(ids(&listed), vec!["ses_b", "ses_a", "ses_c"]);
        assert_eq!(
            listed[0].updated_at.as_second(),
            stamp().as_second() + 60,
            "updated_at is the journal's mtime, not the summary's copy"
        );
    }

    #[tokio::test]
    async fn list_honours_cwd_parent_and_limit() {
        let (_root, store) = root_with();
        aged_session(&store, "ses_here", stamp()).await;
        let mut elsewhere = summary();
        elsewhere.id = SessionId::from_raw("ses_there");
        elsewhere.cwd = "/elsewhere".into();
        store.create(&elsewhere).await.expect("create");
        let mut child = summary();
        child.id = SessionId::from_raw("ses_child");
        child.parent = Some(ParentLink {
            session: SessionId::from_raw("ses_here"),
            item: Some(bingo_sdk::ItemId::from_raw("itm_1")),
        });
        store.create(&child).await.expect("create");

        let by_cwd = SessionFilter {
            cwd: Some("/elsewhere".into()),
            ..SessionFilter::default()
        };
        assert_eq!(
            ids(&store.list(&by_cwd).await.expect("list")),
            vec!["ses_there"]
        );

        let by_parent = SessionFilter {
            parent: Some(SessionId::from_raw("ses_here")),
            ..SessionFilter::default()
        };
        assert_eq!(
            ids(&store.list(&by_parent).await.expect("list")),
            vec!["ses_child"]
        );

        let limited = SessionFilter {
            limit: Some(2),
            ..SessionFilter::default()
        };
        assert_eq!(store.list(&limited).await.expect("list").len(), 2);
    }

    #[tokio::test]
    async fn deleting_every_summary_loses_nothing() {
        let (root, store) = root_with();
        aged_session(&store, "ses_a", stamp()).await;
        aged_session(&store, "ses_b", stamp()).await;
        let before = store.list(&SessionFilter::default()).await.expect("list");

        for session in layout::sessions(store.root()).expect("sessions") {
            std::fs::remove_file(layout::summary(&session.dir)).expect("remove the summary");
        }
        let after = store.list(&SessionFilter::default()).await.expect("list");

        assert_eq!(before, after);
        assert!(root.path().join("sessions/ses_a/summary.json").is_file());
    }

    #[tokio::test]
    async fn a_directory_that_is_not_a_session_is_skipped() {
        let (_root, store) = root_with();
        std::fs::create_dir_all(store.root().join("not-a-session")).expect("mkdir");
        assert!(
            store
                .list(&SessionFilter::default())
                .await
                .expect("list")
                .is_empty()
        );
    }

    #[test]
    fn the_plugin_registers_one_store_under_the_data_directory() {
        let plugin = JsonlStorePlugin::default();
        let env = bingo_sdk::Env::rooted("/home/someone");
        let mut registrar = Registrar::new(
            "bingo.store.jsonl",
            serde_json::Value::Null,
            env,
            Default::default(),
        );
        plugin.register(&mut registrar).expect("register");

        let contributions = registrar.into_contributions();
        assert_eq!(contributions.len(), 2);
        assert!(matches!(contributions[0], Contribution::Store(_)));
        assert_eq!(
            plugin.store.get().expect("the store").root(),
            Path::new("/home/someone/.bingo/data/sessions")
        );
    }
}

async fn off_runtime<T: Send + 'static>(
    work: impl FnOnce() -> Result<T, KernelError> + Send + 'static,
) -> Result<T, KernelError> {
    tokio::task::spawn_blocking(work)
        .await
        .map_err(|error| storage(format!("store worker failed: {error}")))?
}

#[cfg(test)]
mod runtime_contract {
    use super::*;
    use std::sync::Barrier;

    #[tokio::test(flavor = "current_thread")]
    async fn blocking_io_leaves_current_thread_responsive() {
        let runtime = std::thread::current().id();
        let worker = off_runtime(|| Ok(std::thread::current().id()))
            .await
            .unwrap();
        assert_ne!(runtime, worker, "journal I/O must leave the runtime thread");
        let barrier = Arc::new(Barrier::new(2));
        let held = barrier.clone();
        let (entered, started) = tokio::sync::oneshot::channel();
        let write = tokio::spawn(off_runtime(move || {
            let _ = entered.send(());
            held.wait();
            Ok(())
        }));
        started.await.unwrap();
        assert_eq!(
            tokio::spawn(async { "responsive" }).await.unwrap(),
            "responsive"
        );
        tokio::task::spawn_blocking(move || barrier.wait())
            .await
            .unwrap();
        write.await.unwrap().unwrap();
    }
}

#[cfg(test)]
mod inspection_tests;
