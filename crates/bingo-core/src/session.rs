//! The session actor: one mailbox, one journal, one snapshot, one running
//! turn. Every producer — clients, the turn loop, tools — sends a `Msg`; the
//! actor mints `seq`, persists durable frames, folds them with the one reducer
//! and fans them out to bounded subscriber channels. It never awaits a client.

mod catalogue;
mod checkpoint;
mod commands;
mod complete;
mod gate;
mod history;
mod inputs;
mod interactions;
mod invoke;
mod mailbox;
mod past;
mod queue;
mod running;
mod signals;
mod spawn;
mod storage;
mod subscribers;
mod title;

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use bingo_sdk::*;
use jiff::Timestamp;
use serde_json::{Value, json};
use tokio::sync::{mpsc, watch};
use tokio_util::task::TaskTracker;

use commands::Commands;
pub use commands::Services;
pub(crate) use inputs::user_parts;
pub use interactions::INTERACTION_GUARD_MS;
use interactions::Pending;
pub use mailbox::Mailbox;
use mailbox::Msg;
use queue::Queue;
use running::{Ended, Running};
pub(crate) use spawn::{fold_resumed, resume_checkpoint};
pub use spawn::{head_summary, replayed, resume, spawn};
pub use subscribers::SUBSCRIBER_CAPACITY;
use subscribers::Subscribers;

use crate::turn::{HookSet, TurnConfig, TurnKind};

/// How long a stopping actor waits for the work it spawned after its turns
/// (ADR-0008 §7) before it lets go.
pub const AFTER_TURN_DEADLINE: Duration = Duration::from_secs(30);

/// Time for cooperative cleanup after the person stops owned work.
pub const INTERRUPT_GRACE: Duration = Duration::from_secs(2);

/// The start hooks, still running.
type Starting = std::pin::Pin<Box<dyn std::future::Future<Output = ()> + Send>>;

#[derive(PartialEq, Eq)]
enum Flow {
    Continue,
    Stop,
}

struct Actor {
    id: SessionId,
    mailbox: Mailbox,
    rx: mpsc::UnboundedReceiver<Msg>,
    /// The kernel's own view, maintained by the one reducer.
    state: SessionState,
    /// Durable frames, in seq order.
    journal: Vec<Frame>,
    checkpoint_seq: Seq,
    seq: Seq,
    store: Option<Arc<dyn SessionStore>>,
    config: Arc<TurnConfig>,
    subscribers: Subscribers,
    running: Option<Running>,
    queue: Queue,
    commands: Commands,
    gate: gate::InputGate,
    /// Work that outlives a turn: the hooks that run after `TurnCompleted`.
    tracker: TaskTracker,
    cancel: CancellationToken,
    /// Every frame but the deltas for the hooks that observe the session, in
    /// order, on a task of their own; `None` when no hook asked (ADR-0009 §4).
    observed: Option<catalogue::Queues>,
    /// Flipped when the actor is done, for whoever waits on the mailbox.
    done: watch::Sender<bool>,
    pending: HashMap<InteractionId, Pending>,
    /// A close that waits for the running turn to wind down.
    closing: Option<CloseReason>,
    progress_n: u32,
}

impl Actor {
    async fn run(mut self) {
        if self.seq > Seq::ZERO && self.checkpoint_seq == Seq::ZERO {
            checkpoint::write(&self);
        }
        let (held, mut flow) = match self.open().await {
            Ok(starting) => (self.serve_start(starting).await, Flow::Continue),
            Err(_) => (Vec::new(), Flow::Stop),
        };
        for msg in held {
            if self.handle(msg).await == Flow::Stop {
                flow = Flow::Stop;
                break;
            }
        }
        while flow == Flow::Continue {
            let Some(msg) = self.rx.recv().await else {
                break;
            };
            flow = self.handle(msg).await;
        }
        if let Some(running) = self.running.take() {
            running.abandon();
        }
        self.cancel.cancel();
        // Observers may be awaiting a read queued behind close. Release their
        // replies before joining them, since this actor no longer serves mail.
        self.rx.close();
        while let Ok(message) = self.rx.try_recv() {
            drop(message);
        }
        drop(self.observed.take());
        self.tracker.close();
        if tokio::time::timeout(AFTER_TURN_DEADLINE, self.tracker.wait())
            .await
            .is_err()
        {
            tracing::warn!(session = %self.id, "post-turn work did not finish in time");
        }
        let _ = self.done.send(true);
    }

    async fn handle(&mut self, msg: Msg) -> Flow {
        let (calling, msg) = match msg {
            Msg::Calling(calling, msg) => (calling, *msg),
            msg => (Vec::new(), msg),
        };
        invocation::scope(calling, self.dispatch(msg))
            .await
            .unwrap_or(Flow::Stop)
    }

    async fn dispatch(&mut self, msg: Msg) -> Result<Flow, KernelError> {
        match msg {
            Msg::Calling(_, _) => {
                return Err(KernelError::new(
                    ErrorCode::Internal,
                    "nested operation envelope",
                ));
            }
            Msg::CatalogueEvent(event) => {
                if !matches!(event.as_ref(), Event::ConfigChanged { config } if *config == self.state.config)
                {
                    self.publish(*event, None).await?;
                }
            }
            Msg::CompletionReady(completed) => return catalogue::finished(self, *completed).await,
            Msg::Observe(entry, payload) => self.observe_named(entry, payload),
            Msg::Submit { intent, input } => self.submit(intent, input).await?,
            Msg::Gated { intent, verdict } => gate::gated(self, intent, verdict).await?,
            Msg::Deliver {
                intent,
                input,
                delivery,
            } => self.deliver(intent, input, delivery).await?,
            Msg::Extend {
                plugin,
                kind,
                payload,
            } => self.extend(plugin, kind, payload).await?,
            Msg::Invalidated(requests) => {
                self.publish(Event::Invalidated { requests }, None).await?;
            }
            Msg::Signal {
                generation,
                plugin,
                kind,
                payload,
            } => signals::publish(self, plugin, kind, payload, generation).await?,
            Msg::TransferSignals {
                plugin,
                from,
                to,
                reply,
            } => signals::transfer(self, plugin, from, to, reply).await,
            Msg::Interrupt { intent, scope } => self.interrupt(intent, scope).await?,
            Msg::Withdraw {
                intent,
                surface,
                reply,
            } => drop(reply.send(self.withdraw(intent, surface).await)),
            Msg::Answer(answered) => self.answer(answered).await?,
            Msg::Checkpointed { seq, result } => checkpoint::acknowledge(self, seq, result),
            Msg::PastLoaded {
                cut,
                generation,
                transcript,
                request,
            } => past::past_loaded(self, cut, generation, transcript, *request).await,
            Msg::Attach { max_bytes, reply } => past::attach_read(self, max_bytes, reply),
            Msg::TailEvents { reply } => {
                let seq = self.state.seq;
                drop(reply.send((seq, self.subscribe(seq))));
            }
            Msg::EventsSince { since, reply } => drop(reply.send(self.subscribe(since))),
            Msg::History { page, reply } => past::history_read(self, page, reply),
            Msg::ItemForPin {
                id,
                generation,
                reply,
            } => past::pin_read(self, id, generation, reply),
            Msg::Summary { reply } => drop(reply.send(self.stamped(self.state.summary.clone()))),
            Msg::Emit { turn, event } => self.emit(turn, *event).await?,
            Msg::Ask {
                item,
                kind,
                answers,
                reply,
            } => self.open_interaction(item, kind, answers, reply).await?,
            Msg::Absorb { turn, reply } => drop(reply.send(self.absorb(&turn).await?)),
            Msg::TurnFinished { turn, outcome } => return self.turn_finished(turn, outcome).await,
            Msg::Receipt { item, reply } => self.receipt(item, reply).await?,
            Msg::Record { body, reply } => drop(reply.send(self.record(body).await?)),
            Msg::Progress { item, tail } => self.progress(item, tail).await?,
            Msg::CommandFinished { intent, outcome } => {
                catalogue::command_finished(self, intent, outcome).await?
            }
            Msg::CommandReady { intent, outcome } => self.command_finished(intent, outcome).await?,
            Msg::Reconfigure { config } => self.reconfigure(config).await?,
            Msg::Offered { turn, tools } => self.turn_offers(&turn, tools),
            Msg::Complete {
                turn,
                request,
                reply,
            } => complete::complete(self, turn, *request, reply),
            Msg::Charge {
                turn,
                plugin,
                usage,
                reply,
            } => {
                let result = complete::charge(self, turn, plugin, usage).await;
                let _ = reply.send(result);
            }
            Msg::Invoke { call, reply } => self.invoke(call, reply).await?,
            Msg::CallAllowed { item, input } => self.call_allowed(&item, input).await?,
            Msg::CallFinished { outcome } => self.call_finished(outcome).await?,
            Msg::Rewind { to_turn, reply } => past::rewind_read(self, to_turn, reply).await,
            Msg::Compact {
                instructions,
                reply,
            } => drop(reply.send(self.compact(instructions).await)),
            Msg::Close { reason } => return self.close(reason).await,
        }
        Ok(if self.state.closed {
            Flow::Stop
        } else {
            Flow::Continue
        })
    }

    /// A plugin's state, whole, as a durable frame (ADR-0011 §2).
    async fn extend(
        &mut self,
        plugin: String,
        kind: String,
        payload: Value,
    ) -> Result<(), KernelError> {
        let event = Event::Extension {
            plugin,
            kind,
            payload,
        };
        if self.config.hooks.has_links() {
            catalogue::rewrite(self, event);
        } else {
            self.publish(event, None).await?;
        }
        Ok(())
    }

    /// The session's name: the one it was given, else the one its first ask
    /// earns it. A minted name is a name, so once it is published this reads
    /// it back like any other and mints nothing further.
    fn title(&self) -> Option<String> {
        self.state
            .summary
            .title
            .clone()
            .or_else(|| title::first_ask(self.state.transcript.items()).and_then(title::mint))
    }

    /// The mint's own `SessionUpdated`, sent once when the first prose lands
    /// live — the heads of a segment carry it the rest of the time, and no
    /// item or turn is worth a frame of its own (ADR-0005 §5).
    async fn mint_title(&mut self) -> Result<(), KernelError> {
        let minted = self.title();
        if minted == self.state.summary.title {
            return Ok(());
        }
        let summary = SessionSummary {
            title: minted,
            updated_at: Timestamp::now(),
            ..self.state.summary.clone()
        };
        self.publish_summary(summary).await?;
        Ok(())
    }

    /// A summary as it leaves the actor. `busy` is decided here and nowhere
    /// else: a summary travels without the turn it describes — a host listing
    /// carries no `SessionState` — so the actor stamps its own running turn on
    /// it, which is what [`SessionState::busy`] derives from the same fact.
    fn stamped(&self, summary: SessionSummary) -> SessionSummary {
        SessionSummary {
            busy: self.running.is_some(),
            asking: !self.pending.is_empty(),
            ..summary
        }
    }

    /// The one way a summary reaches the journal.
    async fn publish_summary(&mut self, summary: SessionSummary) -> Result<(), KernelError> {
        let summary = self.stamped(summary);
        self.publish(Event::SessionUpdated { summary }, None)
            .await?;
        Ok(())
    }

    /// What a barrier may steer the running turn with: the held prose, and
    /// nothing for a turn that is not the one running.
    async fn absorb(&mut self, turn: &TurnId) -> Result<Vec<(IntentId, Input)>, KernelError> {
        if self.is_running(turn) {
            self.take_steers().await
        } else {
            Ok(Vec::new())
        }
    }

    /// The summary as it stands, saying what the session runs on now. Every
    /// head of a segment carries it, so a resumed session names the model
    /// that will answer it rather than the one that answered it last time.
    fn restated(&self) -> SessionSummary {
        crate::spec::stated(
            &self.config.spec,
            self.config.model.as_ref(),
            SessionSummary {
                updated_at: Timestamp::now(),
                ..self.state.summary.clone()
            },
        )
    }

    /// The head of this segment of the journal: what the session is now.
    /// The start hooks it hands back run while the session opens for reads.
    async fn open(&mut self) -> Result<Starting, KernelError> {
        let summary = SessionSummary {
            title: self.title(),
            ..self.restated()
        };
        self.observe_journal();
        self.publish_summary(summary).await?;
        self.restate_extensions().await?;
        self.refresh_config().await?;
        self.recover().await?;
        self.observe_named(
            bingo_sdk::catalogue::SESSION_START,
            json!({"session":self.id}),
        );
        let cx = self.hook_context();
        Ok(Box::pin(run_session_hooks(
            self.config.hooks.clone(),
            Phase::Start,
            cx,
        )))
    }

    /// While the start hooks run the session answers its summary and holds
    /// everything else: a hook may list the tree it is seating — the host
    /// asks every live actor for its summary, this one included — and what
    /// it seats is there before the first message is read.
    async fn serve_start(&mut self, mut hooks: Starting) -> Vec<Msg> {
        let mut held = Vec::new();
        loop {
            tokio::select! {
                () = &mut hooks => return held,
                msg = self.rx.recv() => match msg {
                    Some(msg) if msg.reads() => {
                        self.handle(msg).await;
                    }
                    Some(msg) => held.push(msg),
                    None => return held,
                },
            }
        }
    }

    /// The head of a segment restates the plugin state the journal already
    /// holds (ADR-0011 §2), so an observer that joins here — a hook after a
    /// restart — folds the same state a client is handed in its snapshot.
    async fn restate_extensions(&mut self) -> Result<(), KernelError> {
        let held: Vec<(String, String, Value)> = self
            .state
            .extensions
            .iter()
            .flat_map(|(plugin, kinds)| {
                kinds
                    .iter()
                    .map(move |(kind, payload)| (plugin.clone(), kind.clone(), payload.clone()))
            })
            .collect();
        for (plugin, kind, payload) in held {
            self.extend(plugin, kind, payload).await?;
        }
        Ok(())
    }

    /// What a client may read of this session's configuration: the kernel's
    /// own keys (ADR-0008 §4) and what the policy says of itself (ADR-0009 §5).
    /// Published when it differs from what the clients already hold.
    async fn refresh_config(&mut self) -> Result<(), KernelError> {
        let policy = &self.config.policy;
        let mut plugins = std::collections::BTreeMap::new();
        let described = policy.describe(&self.id);
        if !described.is_null() {
            plugins.insert(policy.id().to_string(), described);
        }
        let mut kernel =
            json!({ "thinking": self.config.model.as_ref().and_then(|m| m.reasoning) });
        if !self.config.keys.is_empty() {
            kernel["keys"] = json!(self.config.keys);
        }
        let config = ConfigView { kernel, plugins };
        if config != self.state.config {
            if self.config.hooks.has_links() {
                catalogue::rewrite(self, Event::ConfigChanged { config });
            } else {
                self.publish(Event::ConfigChanged { config }, None).await?;
            }
        }
        Ok(())
    }

    /// The end-of-session hooks, off the stopping actor's path. The set is
    /// asked inside the task: a source answers after I/O, and the actor that
    /// is stopping must not wait on one.
    fn session_hooks(&self, phase: Phase) {
        let entry = match phase {
            Phase::Start => bingo_sdk::catalogue::SESSION_START,
            Phase::End => bingo_sdk::catalogue::SESSION_END,
        };
        self.observe_named(entry, json!({"session":self.id}));
        if self.config.hooks.is_empty() {
            return;
        }
        let cx = self.hook_context();
        self.tracker
            .spawn(run_session_hooks(self.config.hooks.clone(), phase, cx));
    }

    /// One ordered task feeds every frame but the deltas to the hooks that
    /// observe the session; publishing never waits on them.
    fn observe_named(&self, entry: LinkPoint, payload: Value) {
        if let Some(observed) = &self.observed {
            let _ = observed.events.send(crate::chain::observe::capture(
                crate::chain::observe::Work::Named(entry, payload, self.hook_context()),
            ));
        }
    }

    fn observe_journal(&mut self) {
        if self.config.hooks.is_empty() {
            return;
        }
        self.observed = Some(catalogue::start(self));
    }

    /// The next turn runs on a config the host rebuilt; the running one
    /// keeps its own. The summary says what changed.
    async fn reconfigure(&mut self, config: Arc<TurnConfig>) -> Result<(), KernelError> {
        self.config = config;
        let summary = self.restated();
        self.publish_summary(summary).await?;
        self.refresh_config().await?;
        Ok(())
    }

    /// Undo a turn and everything after it (ADR-0045 §1): the item that says
    /// so is appended, and `Event::Rewound` takes the items out of every
    /// fold — the shared transcript and every client's
    /// (`SessionState::apply`). The journal itself is never rewritten.
    ///
    /// Refused while a turn runs, whatever the turn is doing: an item the
    /// running turn is still writing is not one this can take back, and a
    /// child agent runs inside its parent's turn.
    async fn rewind(
        &mut self,
        transcript: &Transcript,
        to_turn: &TurnId,
    ) -> Result<u32, KernelError> {
        if self.running.is_some() {
            return Err(KernelError::new(ErrorCode::NotReady, "a turn is running"));
        }
        let dropped = crate::rewind::dropped(transcript.items(), to_turn).ok_or_else(|| {
            KernelError::new(
                ErrorCode::InvalidInput,
                format!("this session has no turn {to_turn}"),
            )
        })?;
        let count = dropped.len() as u32;
        self.record(ItemBody::Rewind {
            to_turn: to_turn.clone(),
            dropped: count,
        })
        .await?;
        self.publish(
            Event::Rewound {
                generation: self.state.transcript.generation() + 1,
                to_turn: to_turn.clone(),
                dropped,
                // The kernel restores no file, so it names none; whoever put
                // them back says so in its own reply (ADR-0045 §3).
                files_restored: Vec::new(),
            },
            None,
        )
        .await?;
        Ok(count)
    }

    /// A turn that only compacts (ADR-0008 §4): refused while one runs, so
    /// that a queued `/compact` never races the turn ahead of it.
    async fn compact(&mut self, instructions: Option<String>) -> Result<(), KernelError> {
        if !self.answers() {
            return Err(KernelError::new(
                ErrorCode::InvalidInput,
                "a log session has nothing to compact",
            ));
        }
        if let Some(model) = self
            .config
            .model
            .as_ref()
            .filter(|m| m.endpoint.holds_context())
        {
            return Err(KernelError::new(
                ErrorCode::InvalidInput,
                format!(
                    "`{}` holds this context and compacts it itself; bingo does not",
                    model.endpoint.id()
                ),
            ));
        }
        if self.running.is_some() {
            return Err(KernelError::new(ErrorCode::NotReady, "a turn is running"));
        }
        self.start_turn(
            Vec::new(),
            TurnOrigin::Auto,
            TurnKind::Compact { instructions },
        )
        .await?;
        Ok(())
    }

    fn hook_context(&self) -> HookContext {
        HookContext {
            item: None,
            host: self.config.host.clone(),
            session: self.id.clone(),
            turn: self.running.as_ref().map(|r| r.turn.clone()),
            cwd: self.config.cwd.clone(),
            provider: self
                .config
                .model
                .as_ref()
                .and_then(|m| m.endpoint.provider()),
            model: self.config.model.as_ref().map(|m| m.id.clone()),
        }
    }

    /// Busy for the queue's purposes: a turn, or a command that holds it.
    fn busy(&self) -> bool {
        self.running.is_some() || self.commands.busy()
    }

    /// Whether a model answers this session. A `Log` session (ADR-0011 §1)
    /// has none: what it is told is the journal's at once, and no turn opens.
    fn answers(&self) -> bool {
        self.config.model.is_some()
    }

    /// A resumed journal may end inside a turn the old process never
    /// finished. Its questions can no longer be answered and its turn is
    /// lost; both are said before anything new happens.
    async fn recover(&mut self) -> Result<(), KernelError> {
        let open: Vec<InteractionId> = self
            .state
            .interactions
            .iter()
            .map(|i| i.id.clone())
            .collect();
        for id in open {
            self.publish(
                Event::InteractionCancelled {
                    id,
                    reason: CancelReason::SessionClosed,
                },
                None,
            )
            .await?;
        }
        if let Some(turn) = self.state.turn.clone() {
            self.publish(
                Event::TurnCompleted {
                    turn: turn.id,
                    status: TurnStatus::Failed {
                        error: KernelError::new(
                            ErrorCode::TurnLost,
                            "the process ended during this turn",
                        ),
                    },
                    usage: turn.usage,
                },
                None,
            )
            .await?;
        }
        Ok(())
    }

    fn is_running(&self, turn: &TurnId) -> bool {
        self.running.as_ref().is_some_and(|r| &r.turn == turn)
    }

    /// An event from the running turn. One from a turn that already ended is
    /// dropped: its seq would land after the `TurnCompleted` that closed it.
    /// A permission receipt means the policy may have installed a rule.
    async fn emit(&mut self, turn: TurnId, event: Event) -> Result<(), KernelError> {
        if !self.is_running(&turn) {
            tracing::warn!(session = %self.id, %turn, "event from a turn that is not running");
            return Ok(());
        }
        let receipt = matches!(
            &event,
            Event::ItemCompleted { item } if matches!(item.body, ItemBody::PermissionReceipt { .. })
        );
        self.publish(event, None).await?;
        if receipt {
            self.refresh_config().await?;
        }
        Ok(())
    }

    /// A running tool's own tail, numbered per session so clients can order it.
    async fn progress(&mut self, item: ItemId, tail: String) -> Result<(), KernelError> {
        self.progress_n += 1;
        self.publish(
            Event::ItemDelta {
                item,
                n: self.progress_n,
                kind: DeltaKind::Tail,
                data: tail,
            },
            None,
        )
        .await?;
        Ok(())
    }

    // ----- publishing -----

    async fn publish(&mut self, event: Event, cause: Option<IntentId>) -> Result<Seq, KernelError> {
        self.seq = self.seq.next();
        let frame = Frame {
            seq: self.seq,
            ts: Timestamp::now(),
            session: self.id.clone(),
            cause,
            event,
        };
        if frame.event.is_durable() {
            if let Some(store) = &self.store
                && let Err(e) = store.append(&self.id, &frame).await
            {
                storage::storage_failed(self, &e);
                return Err(e);
            }
            self.journal.push(frame.clone());
        }
        let checkpoint = matches!(frame.event, Event::Compacted { .. });
        self.broadcast(frame);
        if checkpoint {
            checkpoint::write(self);
        }
        Ok(self.seq)
    }

    fn broadcast(&mut self, frame: Frame) {
        // Observers see notices too; only the high-volume deltas are spared.
        if let Some(observed) = &self.observed
            && !matches!(frame.event, Event::ItemDelta { .. })
        {
            let work =
                crate::chain::observe::Work::Frame(Box::new(frame.clone()), self.hook_context());
            let work = match self.running.as_ref().and_then(|run| run.hooks.get()) {
                Some(hooks) => {
                    crate::chain::observe::Work::Snapshot(Box::new(hooks.clone()), Box::new(work))
                }
                None => work,
            };
            let _ = observed.events.send(crate::chain::observe::capture(work));
        }
        self.state.apply(&frame);
        self.subscribers.fanout(&frame);
    }

    /// The snapshot and its stream are cut by the same mailbox message. A
    /// bounded reader clones only its visible item tail, never the full vector.
    fn attached(
        &mut self,
        max_bytes: Option<usize>,
    ) -> (SessionState, Option<OpenHistory>, FrameStream) {
        let (snapshot, history) = match max_bytes {
            Some(budget) => self.bounded_snapshot(budget),
            None => (self.state.clone(), None),
        };
        let stream = self.subscribe(snapshot.seq);
        (snapshot, history, stream)
    }

    fn bounded_snapshot(&self, budget: usize) -> (SessionState, Option<OpenHistory>) {
        let mut snapshot = SessionState::new(self.state.summary.clone());
        snapshot.seq = self.state.seq;
        snapshot.config = self.state.config.clone();
        snapshot.transcript = self
            .state
            .transcript
            .window(self.state.transcript.items().len());
        snapshot.turn = self.state.turn.clone();
        snapshot.queue = self.state.queue.clone();
        snapshot.interactions = self.state.interactions.clone();
        snapshot.context = self.state.context;
        snapshot.last_turn = self.state.last_turn.clone();
        snapshot.unread = self.state.unread;
        snapshot.closed = self.state.closed;
        snapshot.extensions = self.state.extensions.clone();
        snapshot.signals = self.state.signals.clone();
        // The RPC envelope, id and metadata are added outside the actor. Leave
        // room for those and let the server make the final whole-line decision.
        let available = budget.saturating_sub(4096);
        let mut used = serde_json::to_vec(&snapshot).map_or(available, |bytes| bytes.len());
        let mut start = self.state.transcript.items().len();
        while start > 0 {
            let size = serde_json::to_vec(self.state.transcript.items()[start - 1].as_ref())
                .map_or(available, |bytes| bytes.len());
            if used.saturating_add(size).saturating_add(1) > available {
                break;
            }
            used += size + 1;
            start -= 1;
        }
        snapshot.transcript = self.state.transcript.window(start);
        let history = OpenHistory {
            before: snapshot
                .transcript
                .items()
                .first()
                .map(|item| item.id.clone()),
            has_more: start > 0 || self.state.transcript.history().has_more,
            generation: self.state.transcript.generation(),
        };
        (snapshot, Some(history))
    }

    fn subscribe(&mut self, since: Seq) -> FrameStream {
        if since < self.checkpoint_seq
            && let Some(store) = self.store.clone()
        {
            let live = self.subscribers.add(self.id.clone(), Vec::new());
            return past::catch_up(store, self.id.clone(), since, self.seq.next(), live);
        }
        let replay: Vec<Frame> = self
            .journal
            .iter()
            .filter(|f| f.seq > since)
            .cloned()
            .collect();
        self.subscribers.add(self.id.clone(), replay)
    }

    async fn reject(
        &mut self,
        intent: IntentId,
        code: ErrorCode,
        message: impl Into<String>,
    ) -> Result<(), KernelError> {
        self.publish(
            Event::IntentAck {
                intent: intent.clone(),
                outcome: IntentOutcome::Rejected {
                    error: KernelError::new(code, message),
                },
            },
            Some(intent),
        )
        .await?;
        Ok(())
    }

    async fn applied(&mut self, intent: IntentId, result: Value) -> Result<(), KernelError> {
        self.publish(
            Event::IntentAck {
                intent: intent.clone(),
                outcome: IntentOutcome::Applied { result },
            },
            Some(intent),
        )
        .await?;
        Ok(())
    }

    fn fresh(&self, turn: Option<TurnId>, intent: Option<IntentId>, body: ItemBody) -> Item {
        let now = Timestamp::now();
        Item {
            id: ItemId::mint(),
            turn,
            round: 0,
            status: ItemStatus::Completed,
            started_at: now,
            completed_at: Some(now),
            intent,
            body,
            meta: Default::default(),
        }
    }

    async fn record(&mut self, body: ItemBody) -> Result<ItemId, KernelError> {
        if let Some(id) = self.recorded_denial(&body) {
            self.refresh_config().await?;
            return Ok(id);
        }
        let turn = self.running.as_ref().map(|r| r.turn.clone());
        let item = self.fresh(turn, None, body);
        let id = item.id.clone();
        self.publish(Event::ItemCompleted { item }, None).await?;
        Ok(id)
    }

    async fn turn_finished(&mut self, turn: TurnId, outcome: Ended) -> Result<Flow, KernelError> {
        if !self.is_running(&turn) {
            tracing::warn!(session = %self.id, %turn, "completion from a turn that is not running");
            return Ok(Flow::Continue);
        }
        self.cancel_interactions(CancelReason::TurnEnded).await?;
        let (status, items) = match outcome {
            Ended::Returned(outcome) => (outcome.status, outcome.items),
            Ended::Dropped => {
                let outcome = running::close_dropped_turn(self, &turn).await?;
                (outcome.status, outcome.items)
            }
            Ended::Panicked(panic) => (
                TurnStatus::Failed {
                    error: KernelError::new(
                        ErrorCode::TurnLost,
                        format!("the turn loop panicked: {panic}"),
                    ),
                },
                Vec::new(),
            ),
        };
        catalogue::complete(self, turn, status, items);
        Ok(Flow::Continue)
    }

    /// The turn-end hooks run after the terminal event (ADR-0008 §7), on a
    /// task the actor waits for before it stops.
    fn after_turn(&self, turn: TurnId, items: Vec<Item>) {
        if self.config.hooks.is_empty() {
            return;
        }
        let set = self.config.hooks.clone();
        let cx = HookContext {
            item: None,
            turn: Some(turn.clone()),
            ..self.hook_context()
        };
        self.tracker.spawn(async move {
            for hook in set.at(HookPoint::Turn, None).await {
                hook.on_turn(Phase::End, &turn, &items, &cx).await;
            }
        });
    }

    // ----- closing -----

    async fn close(&mut self, reason: CloseReason) -> Result<Flow, KernelError> {
        if self.state.closed {
            return Ok(Flow::Stop);
        }
        self.cancel.cancel();
        gate::close_gate(self).await?;
        // Before the turn is told to stop, and whether or not there is one: a
        // question opened between turns (ADR-0039 §1) is closed by the same
        // close as any other.
        self.cancel_interactions(CancelReason::SessionClosed)
            .await?;
        if let Some(running) = &self.running {
            running.cancel.cancel();
            self.closing = Some(reason);
            return Ok(Flow::Continue);
        }
        self.finish_close(reason).await
    }

    async fn finish_close(&mut self, reason: CloseReason) -> Result<Flow, KernelError> {
        for (intent, _) in self.queue.drain_all() {
            self.reject(intent, ErrorCode::SessionClosed, "the session is closed")
                .await?;
        }
        let checkpoint = matches!(reason, CloseReason::Client | CloseReason::Shutdown);
        self.publish(Event::SessionClosed { reason }, None).await?;
        if checkpoint {
            checkpoint::write(self);
        }
        self.subscribers.clear();
        self.session_hooks(Phase::End);
        Ok(Flow::Stop)
    }
}

async fn run_session_hooks(hooks: HookSet, phase: Phase, cx: HookContext) {
    for hook in hooks.at(HookPoint::Session, None).await {
        hook.on_session(phase, &cx).await;
    }
}

/// What a panicking task said, for the failure the actor reports in its
/// place: the turn loop's, and a command's.
fn panic_message(payload: Box<dyn std::any::Any + Send>) -> String {
    payload
        .downcast_ref::<&str>()
        .map(|s| (*s).to_string())
        .or_else(|| payload.downcast_ref::<String>().cloned())
        .unwrap_or_else(|| "unknown panic".into())
}

/// Stable, non-security integrity check on the exact serialized UTF-8 bytes.
fn fnv1a64(bytes: &[u8]) -> String {
    let mut hash = 0xcbf2_9ce4_8422_2325u64;
    for byte in bytes {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    format!("{hash:016x}")
}

#[cfg(test)]
mod tests;
