//! The turn state machine. It knows providers, tools, the policy, hooks and
//! contributors as traits and nothing feature-shaped: no inbox, no
//! reminders, no roster. Those are contributors and hooks a plugin registers.
//!
//! ```text
//! Assembling → Streaming → Deciding → Gating → Executing → Barrier → Assembling
//!                 │  overflow → Compacting → Streaming      └ stop / interrupt → Closing
//!                 └ cancel → Closing
//! ```

mod agent;
mod compact;
mod endpoint;
pub use endpoint::Endpoint;
mod config;
mod contributors;
mod host;
mod late;
mod ruler;
mod stream;

use std::sync::Arc;

use bingo_sdk::*;
use jiff::Timestamp;

pub use config::{Breaker, ModelChoice, TurnBudget, TurnConfig};
pub use host::TurnHost;
use late::Late;
pub use late::{CompactorSet, ContributorSet, HookSet, ProviderSet, ToolSet};
use ruler::Ruler;
pub use stream::{MAX_RETRY_DELAY, MAX_SERVER_RETRY_DELAY, backoff};
use stream::{Streamed, stopped_early};

use crate::accumulator::Finished;
use crate::chain::tool::AskVia;
use crate::context::{ContextView, KERNEL_SURFACE, budget, elide, estimate_tokens};
use crate::executor::{self, Gate, PendingCall};
use crate::gate::{GateInput, gate_call};
use crate::models::vision;

pub const INTERRUPTED_MARKER: &str = "[Request interrupted by user]";
pub const CONTINUE_PROMPT: &str = "Continue from where you left off.";
pub const MAX_LENGTH_RECOVERIES: u32 = 3;
const MAX_THRESHOLD_COMPACTIONS: u32 = 2;

pub struct TurnRun {
    pub(crate) hooks: Arc<tokio::sync::OnceCell<HookSet>>,
    pub turn: TurnId,
    /// The actor's folded transcript; item bodies are shared with its snapshot.
    pub transcript: Transcript,
    pub cancel: CancellationToken,
    pub kind: TurnKind,
}

impl TurnRun {
    pub fn new(
        turn: TurnId,
        transcript: &Transcript,
        cancel: CancellationToken,
        kind: TurnKind,
    ) -> Self {
        Self {
            turn,
            transcript: transcript.clone(),
            hooks: Default::default(),
            cancel,
            kind,
        }
    }
}

/// What a turn is for: answering, or only making room (ADR-0008 §4).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TurnKind {
    Respond,
    Compact { instructions: Option<String> },
}

#[derive(Debug, PartialEq)]
pub struct TurnOutcome {
    pub status: TurnStatus,
    /// The transcript as the turn left it, for the hooks that run after it.
    pub items: Vec<Item>,
}

struct Turn<'a> {
    cfg: &'a TurnConfig,
    hooks: HookSet,
    /// The model this turn runs on, resolved once.
    model: ModelChoice,
    host: &'a dyn TurnHost,
    id: TurnId,
    cancel: CancellationToken,
    /// The tools, contributors and strategy this turn runs with, resolved once
    /// when it started (ADR-0009).
    late: Late,
    transcript: Transcript,
    round: u32,
    retries: u32,
    recoveries: u32,
    empty_retry_used: bool,
    overflow_compacted: bool,
    /// Accepted cuts can reissue context without making the whole request fit.
    threshold_compactions: u32,
    hook_cx: HookContext,
    ruler: Ruler,
}

enum Step {
    Assembling,
    Closing(TurnStatus),
}

/// What assembling produced: something to send, or a compaction that has to be
/// assembled around before anything is sent.
enum Assembled {
    Request {
        request: Box<ModelRequest>,
        usage: ContextUsage,
    },
    Compacted,
}

/// One await of a round under the turn's cancel: `None` is a person having
/// stopped the turn, and the work is dropped where it stands.
///
/// Biased for the executor's reason (`executor::run_one`): once a person has
/// stopped the turn nothing is polled again, so what ends a round is always
/// the drop. A cancel and a finishing await racing for the same round would be
/// two endings for one keypress.
async fn racing<T>(cancel: &CancellationToken, work: impl Future<Output = T>) -> Option<T> {
    tokio::select! {
        biased;
        _ = cancel.cancelled() => None,
        done = work => Some(done),
    }
}

/// One item as a turn mints it: now, in this round, under this turn.
pub(crate) fn mint(turn: &TurnId, round: u32, body: ItemBody, status: ItemStatus) -> Item {
    let now = Timestamp::now();
    Item {
        id: ItemId::mint(),
        turn: Some(turn.clone()),
        round,
        status,
        started_at: now,
        completed_at: status.is_terminal().then_some(now),
        intent: None,
        body,
        meta: Default::default(),
    }
}

/// A turn stopped before it had gathered enough to be one. The marker still
/// reaches the transcript and the clients: what a person sees when they press
/// esc does not depend on how far the turn had got.
/// The actor opens no turn on a session nothing answers (ADR-0011 §1); this
/// is the guard, not a path.
fn unanswered(items: Vec<Item>) -> TurnOutcome {
    TurnOutcome {
        status: TurnStatus::Failed {
            error: KernelError::new(ErrorCode::InvalidInput, "this session answers nothing"),
        },
        items,
    }
}

pub async fn run_turn(cfg: &TurnConfig, run: TurnRun, host: &dyn TurnHost) -> TurnOutcome {
    let mut transcript = run.transcript;
    let Some(hooks) = racing(&run.cancel, run.hooks.get_or_init(|| cfg.hooks.snapshot())).await
    else {
        return stopped_early(host, &run.turn, transcript);
    };
    let hooks = hooks.clone();
    let Some(model) = cfg.model.clone() else {
        return unanswered(transcript.model_items().cloned().collect());
    };
    let hook_cx = HookContext {
        item: None,
        host: cfg.host.clone(),
        session: cfg.session.id.clone(),
        turn: Some(run.turn.clone()),
        cwd: cfg.cwd.clone(),
        provider: model.endpoint.provider(),
        model: Some(model.id.clone()),
    };
    let Some(late) = racing(&run.cancel, Late::gather(cfg)).await else {
        return stopped_early(host, &run.turn, transcript);
    };
    host.offered(late.tools.clone());
    for name in &late.shadowed {
        emit_and_fold(
            &mut transcript,
            host,
            Event::Notice {
                level: Level::Warn,
                code: "TOOL_SHADOWED".into(),
                text: format!("a second tool called {name} was dropped"),
            },
        );
    }
    let mut turn = Turn {
        cfg,
        hooks,
        ruler: Ruler::new(
            model.capabilities.context_window,
            model.max_tokens,
            model.endpoint.holds_context(),
        ),
        model,
        host,
        id: run.turn,
        cancel: run.cancel,
        late,
        transcript,
        round: 0,
        retries: 0,
        recoveries: 0,
        empty_retry_used: false,
        overflow_compacted: false,
        threshold_compactions: 0,
        hook_cx,
    };
    let started = turn.start_hooks().await;
    let status = match started {
        Some(()) => match run.kind {
            TurnKind::Respond => match turn.model.endpoint.agent() {
                Some(agent) => turn.agent_turn(agent).await,
                None => turn.respond().await,
            },
            TurnKind::Compact { instructions } => compact::run(&mut turn, instructions).await,
        },
        None => turn.interrupted_status(),
    };
    TurnOutcome {
        status,
        items: turn.transcript.model_items().cloned().collect(),
    }
}

fn emit_and_fold(transcript: &mut Transcript, host: &dyn TurnHost, event: Event) {
    transcript.apply(&event);
    host.emit(event);
}

impl Turn<'_> {
    /// Round trips until one closes the turn.
    async fn respond(&mut self) -> TurnStatus {
        let mut step = Step::Assembling;
        loop {
            step = match step {
                Step::Assembling => self.round_trip().await,
                Step::Closing(status) => break status,
            };
        }
    }

    /// This turn's cancel across one await; see [`racing`].
    async fn racing<T>(&self, work: impl Future<Output = T>) -> Option<T> {
        racing(&self.cancel, work).await
    }

    /// The hooks that claim this point, asked of the one set the config holds.
    async fn hooks(&self, point: HookPoint) -> Vec<Arc<dyn Hook>> {
        self.hooks.at(point, None).await
    }

    /// The `Turn` hooks at the start. A hook that is still thinking when a
    /// person presses esc does not hold the turn open, and neither does the
    /// source that was still being asked for it.
    async fn start_hooks(&self) -> Option<()> {
        let hooks = self.racing(self.hooks(HookPoint::Turn)).await?;
        for hook in hooks {
            self.racing(hook.on_turn(
                Phase::Start,
                &self.id,
                &self.transcript.model_items().cloned().collect::<Vec<_>>(),
                &self.hook_cx,
            ))
            .await?;
        }
        Some(())
    }

    fn fresh(&self, body: ItemBody, status: ItemStatus) -> Item {
        mint(&self.id, self.round, body, status)
    }

    fn record(&mut self, body: ItemBody) -> ItemId {
        let item = self.fresh(body, ItemStatus::Completed);
        let id = item.id.clone();
        emit_and_fold(
            &mut self.transcript,
            self.host,
            Event::ItemCompleted { item },
        );
        id
    }

    fn user_piece(&mut self, parts: Vec<ContentPart>, surface: String) {
        self.record(ItemBody::User {
            parts,
            origin: Origin {
                voice: Some(bingo_sdk::Voice::Quiet),
                surface,
                principal: None,
                conversation: None,
            },
        });
    }

    /// Let the contributors for one placement speak, raced: a contributor
    /// still thinking when a person presses esc does not hold the round open,
    /// and what the ones before it said is dropped with it — nothing half
    /// gathered reaches the transcript.
    async fn gathered(
        &self,
        want: impl Fn(Placement) -> bool,
        usage: &ContextUsage,
    ) -> Option<contributors::Gathered> {
        let query = ContextQuery {
            session: &self.cfg.session,
            host: &self.cfg.host,
            turn: &self.id,
            round: self.round,
            items: &self.transcript.model_items().cloned().collect::<Vec<_>>(),
            usage,
            capabilities: &self.model.capabilities,
            cwd: &self.cfg.cwd,
        };
        self.racing(contributors::gather(&self.late.contributors, want, query))
            .await
    }

    async fn contribute(
        &mut self,
        want: impl Fn(Placement) -> bool,
        usage: &ContextUsage,
    ) -> Option<Vec<SystemBlock>> {
        let gathered = self.gathered(want, usage).await?;
        for (id, e) in gathered.failed {
            self.warn("CONTRIBUTOR_FAILED", format!("{id}: {e}"));
        }
        for (label, parts) in gathered.user {
            self.user_piece(parts, label);
        }
        Some(gathered.system)
    }

    /// A warning on the ephemeral stream: the person sees it, the journal does not.
    fn warn(&mut self, code: &str, text: impl Into<String>) {
        emit_and_fold(
            &mut self.transcript,
            self.host,
            Event::Notice {
                level: Level::Warn,
                code: code.into(),
                text: text.into(),
            },
        );
    }

    fn tool_specs(&self) -> Vec<ToolSpec> {
        self.late.tools.iter().map(|t| t.spec()).collect()
    }

    fn measure(&mut self, system: &[SystemBlock], messages: &[Message]) -> ContextUsage {
        let estimate = estimate_tokens(system, messages, &self.tool_specs());
        let mut usage = self.ruler.measure(estimate);
        if !self.ruler.holds() {
            usage.parts = Some(bingo_sdk::tokens::composition(
                system,
                messages,
                &self.tool_specs(),
            ));
        }
        usage
    }

    fn measure_request(&mut self, request: &ModelRequest) -> ContextUsage {
        let mut usage = self.ruler.measure(estimate_tokens(
            &request.system,
            &request.messages,
            &request.tools,
        ));
        if !self.ruler.holds() {
            usage.parts = Some(bingo_sdk::tokens::composition(
                &request.system,
                &request.messages,
                &request.tools,
            ));
        }
        usage
    }

    /// Only overflow recovery projects away stale results; normal requests
    /// preserve the prefix already sent. The journal items are untouched.
    fn elide_after_overflow(&self, messages: Vec<Message>) -> Vec<Message> {
        if !self.overflow_compacted {
            return messages;
        }
        elide::elide_old_results(
            &messages,
            budget::KEEP_RECENT_AFTER_OVERFLOW,
            budget::ELIDE_MIN_CHARS,
        )
        .unwrap_or(messages)
    }

    /// An exact count when the endpoint offers one and the estimate has
    /// drifted far enough from the last truth to be worth a request.
    async fn recount(&mut self, request: &ModelRequest) -> Option<()> {
        if !self.model.capabilities.count_tokens || !self.ruler.recount_due() {
            return Some(());
        }
        let counted = self
            .racing(self.model.endpoint.count_tokens(request))
            .await?;
        match counted {
            Ok(counted) => self.ruler.counted(counted),
            Err(e) => tracing::debug!(error = %e, "count_tokens unavailable; the estimate stands"),
        }
        Some(())
    }

    /// The person is told once per turn when the window is nearly spent.
    fn warn_once(&mut self, usage: &ContextUsage) {
        if let Some(text) = self.ruler.warning(usage) {
            emit_and_fold(
                &mut self.transcript,
                self.host,
                Event::Notice {
                    level: Level::Warn,
                    code: "CONTEXT_WARNING".into(),
                    text,
                },
            );
        }
    }

    /// One round: assemble, stream, decide, gate, execute, barrier.
    async fn round_trip(&mut self) -> Step {
        if self.cancel.is_cancelled() {
            return self.interrupted();
        }
        if self.cfg.budget.spent(self.round) {
            return Step::Closing(TurnStatus::Failed {
                error: KernelError::new(
                    ErrorCode::TurnBudgetExhausted,
                    format!("{} rounds", self.round),
                ),
            });
        }
        let (request, usage) = match self.assemble().await {
            Some(Assembled::Request { request, usage }) => (*request, usage),
            Some(Assembled::Compacted) => return Step::Assembling,
            None => return self.interrupted(),
        };
        self.host.streaming(true);
        let streamed = self.stream(request.clone()).await;
        self.host.streaming(false);
        let finished = match streamed {
            Streamed::Done(f) => f,
            Streamed::Cancelled => return self.interrupted(),
            Streamed::Failed(error, dropped) => {
                return self.failed_stream(error, dropped, usage, &request).await;
            }
        };
        self.account(&finished, usage);
        match self.decide(&finished).await {
            Some(step) => step,
            None => self.act(&finished).await,
        }
    }

    /// Let the contributors speak, then measure. A first round that is already
    /// over the compaction threshold compacts before it sends anything.
    async fn assemble_request(&mut self) -> Option<(ModelRequest, ContextUsage)> {
        let mut request = ModelRequest {
            model: self.model.id.clone(),
            max_tokens: self.model.max_tokens,
            system: self.cfg.system.clone(),
            messages: ContextView::model_view(&self.transcript),
            tools: self.tool_specs(),
            reasoning: self.model.reasoning,
            session: Some(self.cfg.session.id.clone()),
            provider_options: ProviderMetadata::new(),
        };
        let links = self.hooks.links.clone();
        let cx = self.hook_cx.clone();
        let report = crate::chain::Report::default();
        request.tools = crate::chain::describe::tools(request.tools, &links, &cx, report.clone())
            .await
            .ok()?;
        let result = crate::chain::run::<crate::chain::compose::Compose>(
            &links,
            Next::new(|request| Box::pin(self.compose_request(request))),
            &cx,
            request,
            report.clone(),
        )
        .await;
        for notice in report.take() {
            self.record(notice);
        }
        let request = result.ok()?;
        let usage = self.measure_request(&request);
        self.recount(&request).await?;
        let usage = self.ruler.anchored(usage);
        self.warn_once(&usage);
        Some((request, usage))
    }

    async fn compose_request(
        &mut self,
        mut request: ModelRequest,
    ) -> Result<ModelRequest, KernelError> {
        let preliminary = self.measure_request(&request);
        let original = ContextView::model_view(&self.transcript);
        let extra = self
            .contribute(
                |p| matches!(p, Placement::System { .. } | Placement::RoundStart),
                &preliminary,
            )
            .await
            .ok_or_else(|| KernelError::new(ErrorCode::Interrupted, "composition interrupted"))?;
        request.system.extend(extra);
        if request.messages == original {
            request.messages = ContextView::model_view(&self.transcript);
        }
        let full = self.without_images(request.messages);
        let recent = self.without_old_images(full);
        request.messages = self.elide_after_overflow(recent);
        Ok(request)
    }

    async fn assemble(&mut self) -> Option<Assembled> {
        let (request, usage) = self.assemble_request().await?;
        // A context the endpoint holds is the endpoint's to cut: the kernel
        // has nothing to summarise and nothing to replace (ADR-0055 §1).
        if !self.ruler.holds()
            && usage.used >= self.ruler.lines.trigger
            && self.round == 0
            && self.try_compact(usage, &request).await
        {
            return Some(Assembled::Compacted);
        }
        Some(Assembled::Request {
            request: Box::new(request),
            usage,
        })
    }

    /// A threshold compaction, unless the breaker says the last three bought
    /// nothing; then the turn goes on and the person is told.
    async fn try_compact(&mut self, usage: ContextUsage, request: &ModelRequest) -> bool {
        let Some(compactor) = self.late.compactor.clone() else {
            return false;
        };
        if self.threshold_compactions >= MAX_THRESHOLD_COMPACTIONS {
            self.warn(
                "COMPACTION_SKIPPED",
                "automatic compaction limit reached for this turn; continuing without another summary",
            );
            return false;
        }
        if self.cfg.compaction.tripped() {
            self.warn(
                "COMPACTION_SKIPPED",
                format!(
                    "no summary: the last {} bought nothing; the turn goes on uncompacted",
                    Breaker::TRIP
                ),
            );
            return false;
        }
        self.threshold_compactions += 1;
        self.compact(compactor.as_ref(), CompactReason::Threshold, usage, request)
            .await
    }

    /// A model without vision never sees an image part; the note stands in on
    /// the wire only, and the items keep the image.
    fn without_images(&self, messages: Vec<Message>) -> Vec<Message> {
        if self.model.capabilities.images {
            return messages;
        }
        vision::project_images_out(&messages, &vision::omitted_note(&self.model.id))
            .unwrap_or(messages)
    }

    /// A picture the model has already answered is a note on the wire
    /// (ADR-0061 §1): the items keep it, so a surface still draws it and the
    /// model can read the file again when it wants the picture back.
    fn without_old_images(&self, messages: Vec<Message>) -> Vec<Message> {
        elide::images::elide_old_images(&messages, budget::IMAGE_ROUNDS_KEPT).unwrap_or(messages)
    }

    /// Bill the round to the turn, and say where the context stands after it.
    fn charge(&self, plugin: &str, usage: Usage) {
        if usage != Usage::default() {
            self.host.emit(Event::UsageCharged {
                turn: Some(self.id.clone()),
                plugin: plugin.into(),
                usage,
            });
        }
    }

    fn account(&mut self, finished: &Finished, usage: ContextUsage) {
        let context = self.measured(finished, usage);
        emit_and_fold(
            &mut self.transcript,
            self.host,
            Event::TurnUsage {
                turn: self.id.clone(),
                usage: finished.usage,
                context,
            },
        );
    }

    /// What the round leaves in the context. Where the endpoint holds it, its
    /// own reading is the whole answer (ADR-0055 §2) — and where it said
    /// nothing this round, the last reading still stands, because the turn's
    /// bill is every call of the turn summed and not a measure of anything a
    /// window holds. Otherwise what the provider counted as input beats the
    /// estimate the assembler made.
    fn measured(&mut self, finished: &Finished, usage: ContextUsage) -> ContextUsage {
        if self.ruler.holds() {
            return match finished.context {
                Some((used, window)) => self.ruler.reading(used, window),
                None => usage,
            };
        }
        self.ruler.responded(finished.usage.input_total());
        ContextUsage {
            used: finished.usage.input_total().max(usage.used),
            ..usage
        }
    }

    /// What the response means when it asks for no tool: `None` means it does,
    /// and the round goes on to act.
    async fn decide(&mut self, finished: &Finished) -> Option<Step> {
        if self.host.stopped() && finished.tool_calls.is_empty() {
            return Some(Step::Closing(TurnStatus::Completed));
        }
        if finished.items.is_empty() {
            // One empty response is a provider hiccup; two is an answer.
            if !self.empty_retry_used {
                self.empty_retry_used = true;
                return Some(Step::Assembling);
            }
            return Some(Step::Closing(TurnStatus::Completed));
        }
        if !finished.tool_calls.is_empty() {
            return None;
        }
        if finished.finish_reason.as_ref().map(|r| r.unified) == Some(UnifiedFinish::Length)
            && self.recoveries < MAX_LENGTH_RECOVERIES
        {
            self.recoveries += 1;
            self.user_piece(
                vec![ContentPart::text(CONTINUE_PROMPT)],
                KERNEL_SURFACE.into(),
            );
            self.round += 1;
            return Some(Step::Assembling);
        }
        Some(self.stop_hooks().await)
    }

    /// A `Stop` hook may push the turn into another round instead of ending it.
    async fn stop_hooks(&mut self) -> Step {
        let report = crate::chain::Report::default();
        let result = self
            .racing(crate::chain::stop::dispatch(
                &self.hooks,
                &self.hook_cx,
                report.clone(),
            ))
            .await;
        for notice in report.take() {
            self.record(notice);
        }
        match result {
            Some(Ok(StopResult::Continue { origin, reason })) => {
                self.user_piece(vec![ContentPart::text(reason)], origin);
                self.round += 1;
                Step::Assembling
            }
            Some(Ok(StopResult::Complete)) => Step::Closing(TurnStatus::Completed),
            Some(Err(error)) => Step::Closing(TurnStatus::Failed { error }),
            None => self.interrupted(),
        }
    }

    /// Gate the calls, run them, then hold the barrier before the next round.
    async fn act(&mut self, finished: &Finished) -> Step {
        let calls = self.gate(finished).await;
        let stop_after = self.execute(calls).await;
        if self.cancel.is_cancelled() {
            return self.interrupted();
        }
        if stop_after || self.host.stopped() {
            return Step::Closing(TurnStatus::Completed);
        }
        if self.barrier().await.is_none() {
            return self.interrupted();
        }
        self.round += 1;
        Step::Assembling
    }

    /// The barrier: steering queued during the round joins the transcript, then
    /// the barrier contributors see the round that just happened.
    async fn barrier(&mut self) -> Option<()> {
        let steering = self.racing(self.host.absorb()).await?;
        for (_intent, input) in steering {
            if let Input::Text {
                text,
                images,
                origin,
                ..
            } = input
            {
                self.record(ItemBody::User {
                    parts: crate::session::user_parts(text, images),
                    origin,
                });
            }
        }
        let usage = self.measure(&self.cfg.system, &ContextView::model_view(&self.transcript));
        self.contribute(|p| matches!(p, Placement::Barrier), &usage)
            .await?;
        Some(())
    }

    fn interrupted(&mut self) -> Step {
        Step::Closing(self.interrupted_status())
    }

    /// The marker in the transcript and the status that goes with it: one
    /// keypress, one ending, wherever in the round it arrived.
    fn interrupted_status(&mut self) -> TurnStatus {
        self.record(ItemBody::Interruption {
            marker: INTERRUPTED_MARKER.into(),
        });
        TurnStatus::Interrupted {
            reason: InterruptReason::UserCancel,
        }
    }

    /// The `Compact` hooks at one phase, raced. A stopped turn keeps no
    /// summary, so the strategy is never asked for one it would only discard.
    async fn compact_hooks(&self, phase: Phase) -> Option<()> {
        let hooks = self.racing(self.hooks(HookPoint::Compact)).await?;
        for hook in hooks {
            self.racing(hook.on_compact(phase, &self.hook_cx)).await?;
        }
        Some(())
    }

    /// Ask the strategy for a cut and take it only if it shrinks something;
    /// what it cost is billed either way. Returns whether the items changed.
    async fn compact(
        &mut self,
        compactor: &dyn Compactor,
        reason: CompactReason,
        usage: ContextUsage,
        request: &ModelRequest,
    ) -> bool {
        let started = std::time::Instant::now();
        if self.compact_hooks(Phase::Start).await.is_none() {
            return false;
        }
        let input = CompactInput {
            items: self.transcript.model_items().cloned().collect(),
            usage,
            request: request.clone(),
            reason,
            failures: self.cfg.compaction.failures(),
            keep_budget: self.ruler.lines.keep,
        };
        let report = crate::chain::Report::default();
        let outcome = crate::chain::compact::dispatch(
            &self.hooks.links.clone(),
            &self.hook_cx,
            input,
            compactor,
            &self.model,
            self.cancel.child_token(),
            report.clone(),
        )
        .await;
        for notice in report.take() {
            self.record(notice);
        }
        let _ = self.compact_hooks(Phase::End).await;
        match outcome {
            Ok(c) if c.after < c.before && !self.cancel.is_cancelled() => {
                self.charge("core", c.usage);
                self.cfg.compaction.succeeded();
                self.absorb_compaction(c, started.elapsed());
                self.ruler.forget();
                true
            }
            Ok(c) => {
                self.charge("core", c.usage);
                let failures = self.cfg.compaction.failed();
                self.warn("COMPACTION_USELESS", format!(
                        "the summary would not shrink the context ({} → {} tokens); discarded, {failures} in a row",
                        c.before, c.after
                    ));
                false
            }
            Err(e) => {
                self.charge("core", e.usage);
                self.cfg.compaction.failed();
                self.warn("COMPACTION_FAILED", e.to_string());
                false
            }
        }
    }

    /// Record the summary, announce the cut, and apply it to the transcript.
    fn absorb_compaction(&mut self, c: Compaction, took: std::time::Duration) {
        let replaced = self
            .transcript
            .model_items()
            .take_while(|i| i.id != c.boundary)
            .filter(|i| !c.kept.contains(&i.id))
            .count() as u32;
        let summary = self.record(ItemBody::Compaction {
            summary: c.summary.clone(),
            replaced,
            before: c.before,
            after: c.after,
            duration_ms: took.as_millis() as u64,
        });
        let generation = self.transcript.generation() + 1;
        emit_and_fold(
            &mut self.transcript,
            self.host,
            Event::Compacted {
                generation,
                boundary: c.boundary.clone(),
                summary: summary.clone(),
                kept: c.kept.clone(),
            },
        );
    }

    async fn record_receipt(&mut self, body: Option<ItemBody>) -> bool {
        let Some(body) = body else {
            return true;
        };
        let item = self.fresh(body, ItemStatus::Completed);
        let Ok(item) = self.host.receipt(item).await else {
            self.cancel.cancel();
            return false;
        };
        self.transcript.apply(&Event::ItemCompleted { item });
        true
    }

    async fn gate(&mut self, finished: &Finished) -> Vec<PendingCall> {
        let mut calls = Vec::with_capacity(finished.tool_calls.len());
        // Asked once for the round: the gate does the per-tool matching, and
        // a source is read no more often than the hooks it answers with.
        let hooks = self.hooks.gather().await;
        for (item_id, call) in &finished.tool_calls {
            let tool = self
                .late
                .tools
                .iter()
                .find(|t| t.spec().name == call.name)
                .cloned();
            let prompter = AskVia {
                host: self.host,
                item: item_id.clone(),
            };
            let gated = gate_call(
                GateInput {
                    links: &self.hooks.links.clone(),
                    session: &self.cfg.session.id,
                    cwd: &self.cfg.cwd,
                    item: item_id,
                    call: call.clone(),
                    tool: tool.clone(),
                    policy: self.cfg.policy.as_ref(),
                    hooks: &hooks,
                    hook_cx: &self.hook_cx,
                },
                &prompter,
            )
            .await;
            for notice in gated.notices {
                self.record(notice);
            }
            if !self.record_receipt(gated.receipt).await {
                return Vec::new();
            }
            if let Some(mut item) = self.transcript.get(item_id).cloned()
                && (gated.gate == Gate::Allowed
                    || matches!(&item.body, ItemBody::ToolCall { input, .. } if *input != gated.call.input))
            {
                if gated.gate == Gate::Allowed {
                    item.status = ItemStatus::Running;
                }
                if let ItemBody::ToolCall { input, .. } = &mut item.body {
                    *input = gated.call.input.clone();
                }
                emit_and_fold(&mut self.transcript, self.host, Event::ItemUpdated { item });
            }
            calls.push(PendingCall {
                item: item_id.clone(),
                call: gated.call,
                tool,
                traits: gated.traits,
                gate: gated.gate,
            });
        }
        calls
    }

    /// Returns whether an `after_tool` hook asked to end the turn after this round.
    async fn execute(&mut self, calls: Vec<PendingCall>) -> bool {
        let outcomes = self.run_tools(calls).await;
        self.after_tool_hooks(&outcomes).await
    }

    /// Run the gated calls and fold each result back into the item that made it.
    async fn run_tools(&mut self, calls: Vec<PendingCall>) -> Vec<ToolOutcome> {
        let cfg = self.cfg;
        let session = cfg.session.id.clone();
        let turn = self.id.clone();
        let cancel = self.cancel.clone();

        let report = crate::chain::Report::default();
        let dispatch = executor::Dispatch {
            links: &self.hooks.links,
            cx: &self.hook_cx.clone(),
            report: report.clone(),
        };
        let outcomes = executor::execute_chain(
            calls,
            &cancel,
            |pc| ToolContext {
                call_id: pc.call.call_id.clone(),
                session: session.clone(),
                turn: turn.clone(),
                item: pc.item.clone(),
                cwd: cfg.cwd.clone(),
                cancel: cancel.child_token(),
                env: cfg.env.clone(),
                host: cfg.host.clone(),
                call: cfg.tool_host.clone(),
            },
            |o| {
                if let Some(mut item) = self.transcript.get(&o.item).cloned() {
                    item.status = o.status;
                    item.completed_at = Some(Timestamp::now());
                    if let ItemBody::ToolCall {
                        output,
                        duration_ms,
                        progress,
                        ..
                    } = &mut item.body
                    {
                        *output = Some(o.output.clone());
                        *duration_ms = Some(o.duration_ms);
                        *progress = None;
                    }
                    emit_and_fold(
                        &mut self.transcript,
                        self.host,
                        Event::ItemCompleted { item },
                    );
                }
            },
            Some(&dispatch),
        )
        .await;
        for notice in report.take() {
            self.record(notice);
        }
        outcomes
    }

    /// Let the adapters observe the whole round before honoring a block.
    async fn after_tool_hooks(&self, outcomes: &[ToolOutcome]) -> bool {
        self.racing(crate::chain::tool::after_outcomes(
            outcomes,
            &self.transcript,
            &self.hooks,
            &self.hook_cx,
        ))
        .await
        .unwrap_or(false)
    }
}

#[cfg(test)]
mod tests;
