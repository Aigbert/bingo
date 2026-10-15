//! The kernel this surface is driven against.
//!
//! A `HostApi` and a `SessionPort` that publish frames on demand and keep
//! what was submitted, so a chat's two directions can both be asserted
//! without a kernel in the way.

use std::sync::Arc;
use std::sync::Mutex;
use std::sync::atomic::{AtomicU64, Ordering};

use async_trait::async_trait;
use bingo_sdk::{
    Activation, Answer, Attachment, Catalog, CatalogEntry, Env, ErrorCode, Event, Frame,
    FrameStream, HistoryChunk, HistoryPage, HostHandle, Input, IntentId, InteractionId,
    InterruptScope, KernelError, Seq, SessionChange, SessionHandle, SessionId, SessionPort,
    SessionSelector, SessionSpec, SessionState, SurfaceOptions,
};
use tokio::sync::mpsc;

use crate::fixtures;

fn locked<T>(slot: &Mutex<T>) -> std::sync::MutexGuard<'_, T> {
    slot.lock().unwrap_or_else(|poison| poison.into_inner())
}

/// One session: every attachment gets its own stream, so a chat and a TUI can
/// both be looking at it, which is what the two-surface race needs.
#[derive(Debug, Default)]
pub struct TestSession {
    key: Mutex<String>,
    seq: AtomicU64,
    watchers: Mutex<Vec<mpsc::UnboundedSender<Frame>>>,
    submitted: Mutex<Vec<(IntentId, Input)>>,
    answers: Mutex<Vec<(InteractionId, Answer, Activation)>>,
}

impl TestSession {
    fn key(&self) -> String {
        locked(&self.key).clone()
    }

    fn set_key(&self, key: Option<String>) {
        *locked(&self.key) = key.unwrap_or_default();
    }

    fn attach(&self) -> FrameStream {
        let (publisher, frames) = mpsc::unbounded_channel();
        locked(&self.watchers).push(publisher);
        Box::pin(futures::stream::unfold(frames, |mut frames| async move {
            frames.recv().await.map(|frame| (frame, frames))
        }))
    }

    /// Publish a frame as the kernel would, numbering it as it goes.
    pub fn publish(&self, event: Event) {
        let seq = self.seq.fetch_add(1, Ordering::Relaxed) + 1;
        let frame = fixtures::frame(seq, event);
        locked(&self.watchers).retain(|watcher| watcher.send(frame.clone()).is_ok());
    }

    pub fn intents(&self) -> Vec<IntentId> {
        locked(&self.submitted)
            .iter()
            .map(|(intent, _)| intent.clone())
            .collect()
    }

    pub fn prompts(&self) -> Vec<String> {
        locked(&self.submitted)
            .iter()
            .filter_map(|(_, input)| match input {
                Input::Text { text, .. } => Some(text.clone()),
                Input::Action { .. } => None,
            })
            .collect()
    }

    pub fn origins(&self) -> Vec<bingo_sdk::Origin> {
        locked(&self.submitted)
            .iter()
            .filter_map(|(_, input)| match input {
                Input::Text { origin, .. } => Some(origin.clone()),
                Input::Action { origin, .. } => origin.clone(),
            })
            .collect()
    }

    /// The pictures beside each prompt, in the order they were submitted.
    pub fn pictures(&self) -> Vec<Vec<bingo_sdk::Image>> {
        locked(&self.submitted)
            .iter()
            .filter_map(|(_, input)| match input {
                Input::Text { images, .. } => Some(images.clone()),
                Input::Action { .. } => None,
            })
            .collect()
    }

    pub fn answers(&self) -> Vec<(InteractionId, Answer, Activation)> {
        locked(&self.answers).clone()
    }
}

#[async_trait]
impl SessionPort for TestSession {
    fn submit(&self, intent: IntentId, input: Input) {
        locked(&self.submitted).push((intent, input));
    }

    fn interrupt(&self, _intent: IntentId, _scope: InterruptScope) {}

    fn answer(
        &self,
        _intent: IntentId,
        interaction: InteractionId,
        answer: Answer,
        activation: Activation,
    ) {
        locked(&self.answers).push((interaction, answer, activation));
    }

    async fn history(&self, _page: HistoryPage) -> Result<HistoryChunk, KernelError> {
        Ok(HistoryChunk {
            items: Vec::new(),
            next: None,
            generation: 0,
            oversized: None,
        })
    }

    async fn events_since(&self, _since: Seq) -> Result<FrameStream, KernelError> {
        Ok(self.attach())
    }
}

#[derive(Debug, Default)]
pub struct TestHost {
    sessions: Mutex<Vec<Arc<TestSession>>>,
    /// Every selector `open` was called with, in order.
    opened: Mutex<Vec<SessionSelector>>,
}

impl TestHost {
    pub fn session(&self, key: &str) -> Option<Arc<TestSession>> {
        locked(&self.sessions)
            .iter()
            .find(|session| session.key() == key)
            .cloned()
    }

    pub fn keys(&self) -> Vec<String> {
        locked(&self.sessions)
            .iter()
            .map(|session| session.key())
            .collect()
    }

    pub fn opened(&self) -> Vec<SessionSelector> {
        locked(&self.opened).clone()
    }

    fn attachment(&self, session: Arc<TestSession>) -> Attachment {
        let mut summary = fixtures::summary();
        summary.key = (!session.key().is_empty()).then(|| session.key());
        Attachment {
            session: SessionId::from_raw(fixtures::SESSION),
            snapshot: SessionState::new(summary),
            history: None,
            events: session.attach(),
            handle: SessionHandle(session as Arc<dyn SessionPort>),
        }
    }
}

impl TestHost {
    pub fn scripted(self: &Arc<Self>) -> bingo_sdk::testing::TestHost {
        let host = bingo_sdk::testing::TestHost::default();
        host.on_sessions(|_filter| async move { Ok(Vec::new()) });
        self.script_open(&host);
        host.on_close(|_session, _reason| async move { Ok(()) });
        self.script_reconfigure(&host);
        host.on_delete(|_session| async move { Ok(()) });
        host.on_deliver(|_to, _intent, _input, _delivery| async move {
            unreachable!("this double delivers nothing")
        });
        host.on_extend(|_session, _plugin, _kind, _payload| async move {
            unreachable!("this double extends nothing")
        });
        host.on_signal(|_session, _plugin, _kind, _payload| async move {
            unreachable!("this double signals nothing")
        });
        host.on_catalog(|kind| async move {
            Ok(Catalog {
                kind,
                entries: Vec::<CatalogEntry>::new(),
            })
        });
        host
    }

    fn script_open(self: &Arc<Self>, host: &bingo_sdk::testing::TestHost) {
        let fixture = self.clone();
        host.on_open(move |selector, _who, options| {
            let fixture = fixture.clone();
            async move {
                assert!(options.children, "a chat attaches to the whole tree");
                locked(&fixture.opened).push(selector.clone());
                match selector {
                    SessionSelector::ByKey { key } => fixture
                        .session(&key)
                        .map(|session| fixture.attachment(session))
                        .ok_or_else(|| {
                            KernelError::new(ErrorCode::SessionNotFound, "no such session")
                        }),
                    SessionSelector::Create {
                        spec: SessionSpec { key: Some(key), .. },
                    } => {
                        let session = Arc::new(TestSession {
                            key: Mutex::new(key),
                            ..TestSession::default()
                        });
                        locked(&fixture.sessions).push(Arc::clone(&session));
                        Ok(fixture.attachment(session))
                    }
                    other => panic!("a chat never opens by {other:?}"),
                }
            }
        });
    }

    fn script_reconfigure(self: &Arc<Self>, host: &bingo_sdk::testing::TestHost) {
        let fixture = self.clone();
        host.on_reconfigure(move |_session, change| {
            let fixture = fixture.clone();
            async move {
                if let SessionChange::Key(key) = change
                    && let Some(session) = locked(&fixture.sessions).first()
                {
                    session.set_key(key);
                }
                Ok(())
            }
        });
    }
}

/// A host nothing is ever asked of, for the tests that never get that far.
pub fn nowhere() -> HostHandle {
    Arc::new(TestHost::default()).scripted().handle()
}

pub fn options(cwd: &str) -> SurfaceOptions {
    SurfaceOptions {
        cwd: cwd.into(),
        // The channel surface mints its own keys; the selector is a
        // placeholder it never reads.
        selector: SessionSelector::Latest { cwd: cwd.into() },
        prompt: None,

        env: Arc::new(Env::rooted(cwd)),
    }
}
