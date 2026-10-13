//! What every test in this crate needs: a transcript to pick a turn out of,
//! and a host whose one session is that transcript — `open` hands it back and
//! `rewind` folds the cut into it with the kernel's own reducer, so a test
//! sees what a run would.

use std::path::PathBuf;
use std::sync::{Arc, Mutex, MutexGuard};

use async_trait::async_trait;
use bingo_sdk::{
    Attachment, CommandContext, ContentPart, ErrorCode, Event, Frame, HostHandle, Input, IntentId,
    Item, ItemBody, ItemId, ItemStatus, KernelError, Origin, Seq, SessionHandle, SessionId,
    SessionState, SessionSummary, TurnId, Usage,
};
use jiff::Timestamp;

use crate::store::Checkpoints;

pub(crate) fn session() -> SessionId {
    SessionId::from_raw("ses_one")
}

pub(crate) fn summary() -> SessionSummary {
    SessionSummary {
        tools: None,
        system_extra: None,
        id: session(),
        key: None,
        title: None,
        cwd: "/work".into(),
        parent: None,
        driver: Default::default(),
        model: None,
        provider: None,
        created_at: Timestamp::UNIX_EPOCH,
        updated_at: Timestamp::UNIX_EPOCH,
        usage: Usage::default(),
        busy: false,
        messages: None,
    }
}

fn item(id: &str, turn: &str, body: ItemBody) -> Item {
    Item {
        id: ItemId::from_raw(id),
        turn: Some(TurnId::from_raw(turn)),
        round: 0,
        status: ItemStatus::Completed,
        started_at: Timestamp::UNIX_EPOCH,
        completed_at: Some(Timestamp::UNIX_EPOCH),
        intent: None,
        body,
        meta: Default::default(),
    }
}

fn user(text: &str) -> ItemBody {
    ItemBody::User {
        parts: vec![ContentPart::text(text)],
        origin: Origin::surface("test"),
    }
}

fn assistant(text: &str) -> ItemBody {
    ItemBody::Assistant { text: text.into() }
}

/// Two turns, each an ask and an answer.
pub(crate) fn transcript() -> SessionState {
    let mut state = SessionState::new(summary());
    state.items = vec![
        item("itm_1", "trn_1", user("write the note")),
        item("itm_2", "trn_1", assistant("Written.")),
        item("itm_3", "trn_2", user("and rename it")),
        item("itm_4", "trn_2", assistant("Renamed.")),
    ];
    state
}

#[derive(Default)]
struct Inner {
    state: Mutex<Option<SessionState>>,
    rewound: Mutex<Vec<TurnId>>,
    seq: Mutex<u64>,
}

/// A host that is one session and nothing else.
#[derive(Clone, Default)]
pub(crate) struct Journal(Arc<Inner>);

impl Journal {
    pub(crate) fn holding(state: SessionState) -> Self {
        let journal = Journal::default();
        *journal.state() = Some(state);
        journal
    }

    pub(crate) fn handle(&self) -> HostHandle {
        self.scripted().handle()
    }

    /// Which turns anything asked the kernel to go back to.
    pub(crate) fn rewound(&self) -> Vec<TurnId> {
        self.0
            .rewound
            .lock()
            .unwrap_or_else(|held| held.into_inner())
            .clone()
    }

    pub(crate) fn items(&self) -> Vec<String> {
        match self.state().as_ref() {
            Some(state) => state
                .items
                .iter()
                .map(|item| item.id.as_str().to_string())
                .collect(),
            None => Vec::new(),
        }
    }

    fn state(&self) -> MutexGuard<'_, Option<SessionState>> {
        self.0.state.lock().unwrap_or_else(|held| held.into_inner())
    }

    fn next_seq(&self) -> Seq {
        let mut seq = self.0.seq.lock().unwrap_or_else(|held| held.into_inner());
        *seq += 1;
        Seq(*seq)
    }
}

impl Journal {
    fn scripted(&self) -> bingo_sdk::testing::TestHost {
        let host = bingo_sdk::testing::TestHost::default();
        self.script_sessions(&host);
        self.script_open(&host);
        self.script_rewind(&host);
        host.on_close(
            |_session, _reason| async move { unreachable!("this plugin closes no session") },
        );
        host.on_delete(|_session| async move { unreachable!("this plugin deletes no session") });
        host.on_deliver(|_to, _intent, _input, _delivery| async move {
            unreachable!("this plugin delivers nothing")
        });
        host.on_extend(|_session, _plugin, _kind, _payload| async move {
            unreachable!("this plugin publishes no state")
        });
        host.on_signal(|_session, _plugin, _kind, _payload| async move {
            unreachable!("this plugin signals nothing")
        });
        host.on_catalog(|_kind| async move { unreachable!("this plugin reads no catalog") });
        host.on_gateway_events(|| unreachable!("this plugin watches no gateway"));
        host
    }

    fn script_sessions(&self, host: &bingo_sdk::testing::TestHost) {
        let fixture = self.clone();
        host.on_sessions(move |_filter| {
            let fixture = fixture.clone();
            async move {
                Ok(fixture
                    .state()
                    .as_ref()
                    .map(|state| vec![state.summary.clone()])
                    .unwrap_or_default())
            }
        });
    }

    fn script_open(&self, host: &bingo_sdk::testing::TestHost) {
        let fixture = self.clone();
        host.on_open(move |_selector, _who, _options| {
            let fixture = fixture.clone();
            async move {
                let snapshot = fixture.state().clone().ok_or_else(|| {
                    KernelError::new(ErrorCode::SessionNotFound, "no such session")
                })?;
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

    fn script_rewind(&self, host: &bingo_sdk::testing::TestHost) {
        let fixture = self.clone();
        host.on_rewind(move |session, to_turn| {
            let fixture = fixture.clone();
            async move {
                let session = &session;
                let to_turn = &to_turn;
                let seq = fixture.next_seq();
                let mut held = fixture.state();
                let state = held.as_mut().ok_or_else(|| {
                    KernelError::new(ErrorCode::SessionNotFound, "no such session")
                })?;
                let at = state
                    .items
                    .iter()
                    .position(|item| item.turn.as_ref() == Some(to_turn))
                    .ok_or_else(|| KernelError::new(ErrorCode::InvalidInput, "no such turn"))?;
                let dropped: Vec<ItemId> = state.items[at..].iter().map(|i| i.id.clone()).collect();
                let count = dropped.len() as u32;
                state.apply(&Frame {
                    seq,
                    ts: Timestamp::UNIX_EPOCH,
                    session: session.clone(),
                    cause: None,
                    event: Event::Rewound {
                        generation: state.history_generation + 1,
                        to_turn: to_turn.clone(),
                        dropped,
                        files_restored: Vec::new(),
                    },
                });
                fixture
                    .0
                    .rewound
                    .lock()
                    .unwrap_or_else(|held| held.into_inner())
                    .push(to_turn.clone());
                Ok(count)
            }
        });
    }
}

/// A handle nothing is written to: this plugin reaches its session through
/// `open` and `rewind`, never through a client port.
struct Deaf;

#[async_trait]
impl bingo_sdk::SessionPort for Deaf {
    fn submit(&self, _intent: IntentId, _input: Input) {
        unreachable!("this plugin submits nothing")
    }

    fn interrupt(&self, _intent: IntentId, _scope: bingo_sdk::InterruptScope) {
        unreachable!("this plugin interrupts nothing")
    }

    fn answer(
        &self,
        _intent: IntentId,
        _interaction: bingo_sdk::InteractionId,
        _answer: bingo_sdk::Answer,
        _activation: bingo_sdk::Activation,
    ) {
        unreachable!("this plugin answers nothing")
    }

    async fn history(
        &self,
        _page: bingo_sdk::HistoryPage,
    ) -> Result<bingo_sdk::HistoryChunk, KernelError> {
        unreachable!("this plugin pages no history")
    }

    async fn events_since(&self, _since: Seq) -> Result<bingo_sdk::FrameStream, KernelError> {
        unreachable!("this plugin re-subscribes to nothing")
    }
}

/// A session, a store under a scratch data directory, and the working tree
/// the files live in.
pub(crate) struct Fixture {
    pub(crate) journal: Journal,
    pub(crate) store: Arc<Checkpoints>,
    pub(crate) cwd: PathBuf,
    _home: tempfile::TempDir,
}

impl Fixture {
    pub(crate) fn new() -> Self {
        let home = tempfile::tempdir().expect("a scratch home");
        let cwd = home.path().join("work");
        std::fs::create_dir_all(&cwd).expect("a working tree");
        Self {
            journal: Journal::holding(transcript()),
            store: Arc::new(Checkpoints::new(&home.path().join("data"))),
            cwd,
            _home: home,
        }
    }

    pub(crate) fn command(&self) -> CommandContext {
        CommandContext {
            session: session(),
            cwd: self.cwd.clone(),
            host: self.journal.handle(),
        }
    }

    /// A file in the working tree, and its bytes kept as this turn found it.
    pub(crate) fn edit(&self, turn: &str, name: &str, after: &[u8]) -> PathBuf {
        let path = self.cwd.join(name);
        self.store
            .snapshot(&session(), &TurnId::from_raw(turn), &path)
            .expect("a snapshot");
        std::fs::write(&path, after).expect("the edit itself");
        path
    }
}
