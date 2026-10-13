//! What every test in this crate needs: a host whose sessions are their
//! journals — `open` hands back the state folded so far and `extend` folds
//! one more frame into it with the kernel's own reducer, so a test sees
//! exactly what a run would — and the contexts the sdk hands a tool, a
//! command and a contributor.

use std::path::PathBuf;
use std::sync::{Arc, Mutex, MutexGuard};

use async_trait::async_trait;
use bingo_sdk::{
    Activation, Answer, AnswerSpec, Attachment, CancellationToken, CommandContext, ContextQuery,
    ContextUsage, Driver, Env, ErrorCode, Event, Frame, FrameStream, HistoryChunk, HistoryPage,
    HostHandle, Input, IntentId, InteractionId, InteractionKind, InterruptScope, Item, ItemBody,
    ItemId, KernelError, ModelCapabilities, ParentLink, Prompter, Seq, SessionHandle, SessionId,
    SessionPort, SessionSelector, SessionState, SessionSummary, ToolContext, ToolHost, ToolOutput,
    TurnId, Usage,
};
use jiff::Timestamp;

#[derive(Default)]
struct Inner {
    sessions: Mutex<Vec<SessionState>>,
    /// The kernel's frames arrive in one order; the reducer refuses a frame
    /// it has already seen, so a double must number them as it does.
    seq: Mutex<u64>,
    /// How often the tree was listed, so a test can prove a private list
    /// walks nothing.
    listings: Mutex<usize>,
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

    /// A session with an empty journal, as a surface opens one: no parent and
    /// no name of its own, which is what a person's own session is.
    pub(crate) fn session(&self) -> SessionId {
        self.open(None, None, Driver::default())
    }

    /// A session that answers, under `parent` and by that name.
    pub(crate) fn child(&self, parent: &SessionId, title: &str) -> SessionId {
        self.open(Some(parent), Some(title), Driver::default())
    }

    /// A room: a session nobody answers for, which is what a board hangs in.
    pub(crate) fn room(&self, parent: &SessionId, title: &str) -> SessionId {
        self.open(Some(parent), Some(title), Driver::Log)
    }

    fn open(&self, parent: Option<&SessionId>, title: Option<&str>, driver: Driver) -> SessionId {
        let id = SessionId::mint();
        let mut summary = summary(&id);
        summary.title = title.map(str::to_string);
        summary.driver = driver;
        summary.parent = parent.map(|session| ParentLink {
            session: session.clone(),
            item: None,
        });
        self.states().push(SessionState::new(summary));
        id
    }

    /// How often anything asked the host for the session tree.
    pub(crate) fn session_reads(&self) -> usize {
        *self
            .0
            .listings
            .lock()
            .unwrap_or_else(|held| held.into_inner())
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
}

impl Journals {
    fn scripted(&self) -> bingo_sdk::testing::TestHost {
        let host = bingo_sdk::testing::TestHost::default();
        self.script_sessions(&host);
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

    fn script_sessions(&self, host: &bingo_sdk::testing::TestHost) {
        let fixture = self.clone();
        host.on_sessions(move |filter| {
            let fixture = fixture.clone();
            async move {
                *fixture
                    .0
                    .listings
                    .lock()
                    .unwrap_or_else(|held| held.into_inner()) += 1;
                Ok(fixture
                    .states()
                    .iter()
                    .map(|state| state.summary.clone())
                    .filter(|summary| {
                        filter.parent.as_ref().is_none_or(|parent| {
                            summary
                                .parent
                                .as_ref()
                                .is_some_and(|link| &link.session == parent)
                        })
                    })
                    .collect())
            }
        });
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
                let frame = Frame {
                    seq: fixture.next_seq(),
                    ts: Timestamp::UNIX_EPOCH,
                    session: session.clone(),
                    cause: None,
                    event: Event::Extension {
                        plugin: plugin.to_string(),
                        kind: kind.to_string(),
                        payload,
                    },
                };
                let mut states = fixture.states();
                let state = states
                    .iter_mut()
                    .find(|state| &state.summary.id == session)
                    .ok_or_else(|| {
                        KernelError::new(ErrorCode::SessionNotFound, "no such session")
                    })?;
                state.apply(&frame);
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
                let frame = Frame {
                    seq: fixture.next_seq(),
                    ts: Timestamp::UNIX_EPOCH,
                    session: session.clone(),
                    cause: None,
                    event: Event::Signal {
                        plugin: plugin.to_string(),
                        kind: kind.to_string(),
                        payload,
                    },
                };
                let mut states = fixture.states();
                let state = states
                    .iter_mut()
                    .find(|state| &state.summary.id == session)
                    .ok_or_else(|| {
                        KernelError::new(ErrorCode::SessionNotFound, "no such session")
                    })?;
                state.apply(&frame);
                Ok(())
            }
        });
    }
}

/// A handle nothing is written to: this plugin reaches a session through
/// `open` and `extend`, never through a client port.
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

/// What a tool is handed as its call's own; a tasks tool asks nobody
/// anything and records nothing outside its own result.
struct Silent;

#[async_trait]
impl Prompter for Silent {
    async fn ask(
        &self,
        _kind: InteractionKind,
        _answers: Vec<AnswerSpec>,
    ) -> Result<Answer, KernelError> {
        unreachable!("a tasks tool asks nobody anything")
    }
}

#[async_trait]
impl ToolHost for Silent {
    fn progress(&self, _item: &ItemId, _tail: String) {}

    async fn record(&self, _body: ItemBody) -> Result<ItemId, KernelError> {
        unreachable!("a tasks tool records nothing of its own")
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

/// What a contributor is asked: the session it is contributing to, and the
/// host it reads the journal through.
pub(crate) struct Asked {
    session: SessionSummary,
    turn: TurnId,
    items: Vec<Item>,
    usage: ContextUsage,
    capabilities: ModelCapabilities,
    cwd: PathBuf,
    host: HostHandle,
}

impl Asked {
    pub(crate) fn new(session: &SessionId, journals: &Journals) -> Self {
        Self {
            session: summary(session),
            turn: TurnId::from_raw("trn_test"),
            items: Vec::new(),
            usage: ContextUsage {
                used: 0,
                window: 100_000,
                trigger: 90_000,
            },
            capabilities: ModelCapabilities {
                context_window: 100_000,
                max_output: 8_000,
                images: false,
                reasoning: false,
                count_tokens: false,
                caching: false,
            },
            cwd: PathBuf::from("/work/project"),
            host: journals.handle(),
        }
    }

    pub(crate) fn query(&self) -> ContextQuery<'_> {
        ContextQuery {
            session: &self.session,
            host: &self.host,
            turn: &self.turn,
            round: 0,
            items: &self.items,
            usage: &self.usage,
            capabilities: &self.capabilities,
            cwd: &self.cwd,
        }
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

/// The text a tool answered with, as the model would read it.
pub(crate) fn text(out: &ToolOutput) -> String {
    out.parts
        .iter()
        .filter_map(bingo_sdk::ContentPart::as_text)
        .collect()
}
