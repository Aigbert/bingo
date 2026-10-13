//! A one-session host that folds extension frames with the SDK reducer.

use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, MutexGuard};

use async_trait::async_trait;
use bingo_sdk::{
    Attachment, ContextContributor, ContextPiece, Event, Frame, HostHandle, Input, IntentId,
    KernelError, Seq, SessionHandle, SessionId, SessionSelector, SessionState,
};

use crate::query::Asked;

struct Inner {
    state: Mutex<SessionState>,
    opening: AtomicBool,
}

#[derive(Clone)]
pub(crate) struct Journal(Arc<Inner>);

impl Journal {
    pub(crate) fn at(cwd: &Path) -> Self {
        Self::holding(SessionState::new(Asked::at(cwd).query().session.clone()))
    }

    pub(crate) fn holding(state: SessionState) -> Self {
        Self(Arc::new(Inner {
            state: Mutex::new(state),
            opening: AtomicBool::new(false),
        }))
    }

    pub(crate) fn state(&self) -> MutexGuard<'_, SessionState> {
        self.0.state.lock().expect("the journal lock")
    }

    pub(crate) fn opening(&self, opening: bool) {
        self.0.opening.store(opening, Ordering::SeqCst);
    }

    pub(crate) fn handle(&self) -> HostHandle {
        self.scripted().handle()
    }

    pub(crate) fn apply(&self, event: Event) {
        let mut state = self.state();
        let frame = Frame {
            seq: Seq(state.seq.0 + 1),
            ts: jiff::Timestamp::UNIX_EPOCH,
            session: state.summary.id.clone(),
            cause: None,
            event,
        };
        state.apply(&frame);
    }

    pub(crate) async fn contribute(
        &self,
        contributor: &dyn ContextContributor,
        cwd: &Path,
    ) -> Vec<ContextPiece> {
        let asked = Asked::at(cwd);
        let host = self.handle();
        let mut query = asked.query();
        query.host = &host;
        contributor.contribute(query).await.expect("context")
    }
}

impl Journal {
    fn scripted(&self) -> bingo_sdk::testing::TestHost {
        let host = bingo_sdk::testing::TestHost::default();
        self.script_open(&host);
        self.script_extend(&host);
        self.script_signal(&host);
        host
    }

    fn script_open(&self, host: &bingo_sdk::testing::TestHost) {
        let fixture = self.clone();
        host.on_open(move |selector, _who, _options| {
            let fixture = fixture.clone();
            async move {
                assert!(
                    !fixture.0.opening.load(Ordering::SeqCst),
                    "startup must never open itself"
                );
                let snapshot = fixture.state().clone();
                assert_eq!(
                    selector,
                    SessionSelector::ById {
                        id: snapshot.summary.id.clone()
                    }
                );
                Ok(Attachment {
                    session: snapshot.summary.id.clone(),
                    snapshot,
                    history: None,
                    events: Box::pin(futures::stream::empty()),
                    handle: SessionHandle(Arc::new(Deaf)),
                })
            }
        });
    }

    fn script_extend(&self, host: &bingo_sdk::testing::TestHost) {
        let fixture = self.clone();
        host.on_extend(move |session, plugin, kind, payload| {
            let fixture = fixture.clone();
            async move {
                let session = &session;
                let plugin = &plugin;
                let kind = &kind;
                assert_eq!(session, &fixture.state().summary.id);
                fixture.apply(Event::Extension {
                    plugin: plugin.into(),
                    kind: kind.into(),
                    payload,
                });
                Ok(())
            }
        });
    }

    fn script_signal(&self, host: &bingo_sdk::testing::TestHost) {
        let fixture = self.clone();
        host.on_signal(move |session, plugin, kind, payload| {
            let fixture = fixture.clone();
            async move {
                let session = &session;
                let plugin = &plugin;
                let kind = &kind;
                assert_eq!(session, &fixture.state().summary.id);
                fixture.apply(Event::Signal {
                    plugin: plugin.into(),
                    kind: kind.into(),
                    payload,
                });
                Ok(())
            }
        });
    }
}

struct Deaf;

#[async_trait]
impl bingo_sdk::SessionPort for Deaf {
    fn submit(&self, _: IntentId, _: Input) {
        unreachable!("a contributor submits nothing")
    }

    fn interrupt(&self, _: IntentId, _: bingo_sdk::InterruptScope) {
        unreachable!("a contributor interrupts nothing")
    }

    fn answer(
        &self,
        _: IntentId,
        _: bingo_sdk::InteractionId,
        _: bingo_sdk::Answer,
        _: bingo_sdk::Activation,
    ) {
        unreachable!("a contributor answers nothing")
    }

    async fn history(
        &self,
        _: bingo_sdk::HistoryPage,
    ) -> Result<bingo_sdk::HistoryChunk, KernelError> {
        unreachable!("a contributor pages no history")
    }

    async fn events_since(&self, _: Seq) -> Result<bingo_sdk::FrameStream, KernelError> {
        unreachable!("a contributor subscribes to no events")
    }
}
