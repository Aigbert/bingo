//! What every test in this crate needs: a host whose sessions are their
//! journals — `open` hands back the state folded so far and `extend` folds
//! one more frame into it with the kernel's own reducer — plus the list of
//! what was signalled, which the reducer coalesces away on purpose.

use std::path::PathBuf;
use std::sync::{Arc, Mutex, MutexGuard};

use async_trait::async_trait;
use bingo_sdk::{
    Activation, Answer, AnswerSpec, Attachment, CancellationToken, CommandContext, Env, ErrorCode,
    Event, Frame, FrameStream, HistoryChunk, HistoryPage, HostHandle, Input, IntentId,
    InteractionId, InteractionKind, InterruptScope, ItemBody, ItemId, KernelError, Prompter, Seq,
    SessionHandle, SessionId, SessionPort, SessionSelector, SessionState, SessionSummary,
    ToolContext, ToolHost, TurnId, Usage,
};
use jiff::Timestamp;
use serde_json::Value;

#[derive(Default)]
struct Inner {
    sessions: Mutex<Vec<SessionState>>,
    /// Every signal in the order it was published: the reducer keeps only the
    /// latest per kind, which is the point, so a test that watches a bar move
    /// has to be told each frame as it goes.
    signals: Mutex<Vec<(String, Value)>>,
    /// The kernel's frames arrive in one order; the reducer refuses a frame
    /// it has already seen, so a double must number them as it does.
    seq: Mutex<u64>,
}

/// A host that keeps one folded state per session and nothing else.
#[derive(Clone, Default)]
pub(crate) struct Journals(Arc<Inner>);

impl Journals {
    pub(crate) fn new() -> Self {
        Self::default()
    }

    pub(crate) fn handle(&self) -> HostHandle {
        self.scripted().handle()
    }

    /// A session with an empty journal, as a surface opens one.
    pub(crate) fn session(&self) -> SessionId {
        let id = SessionId::mint();
        self.states().push(SessionState::new(summary(&id)));
        id
    }

    /// Every signal published, kind and payload, in order.
    pub(crate) fn signals(&self) -> Vec<(String, Value)> {
        self.0
            .signals
            .lock()
            .unwrap_or_else(|held| held.into_inner())
            .clone()
    }

    /// What the journals hold, which a signal never reaches.
    pub(crate) fn extensions(&self) -> Vec<String> {
        self.states()
            .iter()
            .flat_map(|state| state.extensions.keys().cloned().collect::<Vec<_>>())
            .collect()
    }

    fn states(&self) -> MutexGuard<'_, Vec<SessionState>> {
        self.0
            .sessions
            .lock()
            .unwrap_or_else(|held| held.into_inner())
    }

    fn next_seq(&self) -> Seq {
        let mut seq = self.0.seq.lock().unwrap_or_else(|held| held.into_inner());
        *seq += 1;
        Seq(*seq)
    }

    fn fold(&self, session: &SessionId, event: Event) -> Result<(), KernelError> {
        let frame = Frame {
            seq: self.next_seq(),
            ts: Timestamp::UNIX_EPOCH,
            session: session.clone(),
            cause: None,
            event,
        };
        let mut states = self.states();
        let state = states
            .iter_mut()
            .find(|state| &state.summary.id == session)
            .ok_or_else(|| KernelError::new(ErrorCode::SessionNotFound, "no such session"))?;
        state.apply(&frame);
        Ok(())
    }
}

impl Journals {
    fn scripted(&self) -> bingo_sdk::testing::TestHost {
        let host = bingo_sdk::testing::TestHost::default();
        host.on_sessions(
            |_filter| async move { unreachable!("this plugin reads no session list") },
        );
        self.script_open(&host);
        host.on_close(
            |_session, _reason| async move { unreachable!("this plugin closes no session") },
        );
        host.on_delete(|_session| async move { unreachable!("this plugin deletes no session") });
        host.on_deliver(|_to, _intent, _input, _delivery| async move {
            unreachable!("this plugin delivers nothing")
        });
        self.script_extend(&host);
        self.script_signal(&host);
        host.on_catalog(|_kind| async move { unreachable!("this plugin reads no catalog") });
        host.on_gateway_events(|| unreachable!("this plugin watches no gateway"));
        host
    }

    fn script_open(&self, host: &bingo_sdk::testing::TestHost) {
        let fixture = self.clone();
        host.on_open(move |selector, _who, _options| {
            let fixture = fixture.clone();
            async move {
                let SessionSelector::ById { id } = selector else {
                    unreachable!("this plugin opens a session by id")
                };
                let states = fixture.states();
                let snapshot = states
                    .iter()
                    .find(|state| state.summary.id == id)
                    .ok_or_else(|| KernelError::new(ErrorCode::SessionNotFound, "no such session"))?
                    .clone();
                Ok(Attachment {
                    session: id,
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
                fixture.fold(
                    session,
                    Event::Extension {
                        plugin: plugin.to_string(),
                        kind: kind.to_string(),
                        payload,
                    },
                )
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
                fixture
                    .0
                    .signals
                    .lock()
                    .unwrap_or_else(|held| held.into_inner())
                    .push((kind.to_string(), payload.clone()));
                fixture.fold(
                    session,
                    Event::Signal {
                        plugin: plugin.to_string(),
                        kind: kind.to_string(),
                        payload,
                    },
                )
            }
        });
    }
}

/// A handle nothing is written to: this plugin reaches a session through
/// `open`, `extend` and `signal`, never through a client port.
struct Deaf;

#[async_trait]
impl SessionPort for Deaf {
    fn submit(&self, _intent: IntentId, _input: Input) {
        unreachable!("this plugin submits nothing")
    }

    fn interrupt(&self, _intent: IntentId, _scope: InterruptScope) {
        unreachable!("this plugin interrupts nothing")
    }

    fn answer(
        &self,
        _intent: IntentId,
        _interaction: InteractionId,
        _answer: Answer,
        _activation: Activation,
    ) {
        unreachable!("this plugin answers nothing")
    }

    async fn history(&self, _page: HistoryPage) -> Result<HistoryChunk, KernelError> {
        unreachable!("this plugin pages no history")
    }

    async fn events_since(&self, _since: Seq) -> Result<FrameStream, KernelError> {
        unreachable!("this plugin re-subscribes to nothing")
    }
}

/// What a tool is handed as its call's own; this one asks nobody anything and
/// records nothing outside its own result.
struct Silent;

#[async_trait]
impl Prompter for Silent {
    async fn ask(
        &self,
        _kind: InteractionKind,
        _answers: Vec<AnswerSpec>,
    ) -> Result<Answer, KernelError> {
        unreachable!("this tool asks nobody anything")
    }
}

#[async_trait]
impl ToolHost for Silent {
    fn progress(&self, _item: &ItemId, _tail: String) {}

    async fn record(&self, _body: ItemBody) -> Result<ItemId, KernelError> {
        unreachable!("this tool records nothing of its own")
    }
}

pub(crate) fn tool_context(session: &SessionId, journals: &Journals) -> ToolContext {
    ToolContext {
        call_id: "call_test".into(),
        session: session.clone(),
        turn: TurnId::from_raw("trn_test"),
        item: ItemId::from_raw("itm_test"),
        cwd: PathBuf::from("/work/project"),
        cancel: CancellationToken::new(),
        env: Arc::new(Env::rooted("/nowhere")),
        host: journals.handle(),
        call: Arc::new(Silent),
    }
}

pub(crate) fn command_context(session: &SessionId, journals: &Journals) -> CommandContext {
    CommandContext {
        cancel: bingo_sdk::CancellationToken::new(),
        session: session.clone(),
        cwd: PathBuf::from("/work/project"),
        host: journals.handle(),
    }
}

fn summary(id: &SessionId) -> SessionSummary {
    SessionSummary {
        tools: None,
        system_extra: None,
        id: id.clone(),
        key: None,
        title: None,
        cwd: "/work/project".into(),
        parent: None,
        driver: Default::default(),
        model: None,
        provider: None,
        created_at: Timestamp::UNIX_EPOCH,
        updated_at: Timestamp::UNIX_EPOCH,
        usage: Usage::default(),
        busy: false,
        asking: false,
        messages: None,
    }
}

#[tokio::test]
async fn m148_board_places_its_table_and_buttons_on_its_recorded_row() {
    use bingo_sdk::{Command, Item, ItemStatus, Level};
    let journals = Journals::new();
    let session = journals.session();
    let id = ItemId::from_raw("itm_board");
    journals
        .fold(
            &session,
            Event::ItemCompleted {
                item: Item {
                    id: id.clone(),
                    turn: None,
                    round: 0,
                    status: ItemStatus::Completed,
                    started_at: Timestamp::UNIX_EPOCH,
                    completed_at: Some(Timestamp::UNIX_EPOCH),
                    intent: None,
                    meta: Default::default(),
                    body: ItemBody::Notice {
                        level: Level::Info,
                        code: "DEMO_BOARD".into(),
                        text: "Board".into(),
                    },
                },
            },
        )
        .unwrap();
    crate::command::SHOW
        .run("", &command_context(&session, &journals))
        .await
        .unwrap();
    let states = journals.states();
    let slots = bingo_sdk::slot::placed(&states[0]);
    let bingo_sdk::slot::Slot::Row {
        item,
        body: Some(bingo_sdk::View::Table { .. }),
        actions,
        ..
    } = &slots.first().expect("board row").slot
    else {
        panic!("table row")
    };
    assert_eq!(item, &id);
    assert_eq!(
        actions
            .iter()
            .map(|action| action.label.as_str())
            .collect::<Vec<_>>(),
        ["Tick", "Reset"]
    );
}

#[tokio::test]
async fn m148_a_fresh_board_records_a_transcript_anchor() {
    use bingo_sdk::{Command, CommandOutcome};
    let journals = Journals::new();
    let session = journals.session();
    let outcome = crate::command::SHOW
        .run("", &command_context(&session, &journals))
        .await
        .unwrap();
    assert!(
        matches!(outcome, CommandOutcome::Record { body: ItemBody::Notice { ref code, .. } } if code == "DEMO_BOARD")
    );
}
