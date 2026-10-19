//! The one vocabulary that crosses kernel → client. A `Frame` is one line of a
//! session's journal; `Event` is everything that can happen in a session; an
//! `Item` is one unit in transcript order. Deltas, notices and lag markers are
//! ephemeral (never journaled); everything else is durable.

mod hunks;
mod interaction;
mod raw;
mod record;

pub use interaction::{
    Activation, Answer, AnswerRole, AnswerSpec, CancelReason, Caution, Interaction,
    InteractionKind, LoginFlow, Preview, Question, QuestionOption, ResolvedBy, Rung, Stakes,
};
pub use record::UnknownRecord;

use std::collections::BTreeMap;

use jiff::Timestamp;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::contributor::CONTRIBUTOR_PREFIX;
use crate::error::KernelError;
use crate::ids::{IntentId, InteractionId, ItemId, Seq, SessionId, TurnId};
use crate::model::{ContentPart, ProviderMetadata, Usage};
use crate::view::{Tone, View};

fn is_false(b: &bool) -> bool {
    !*b
}

/// Namespaces beginning with `_` hold internal extension state. Journals and
/// raw RPC retain it, but ordinary surfaces must not offer it as a panel/card.
pub fn is_internal_extension(plugin: &str) -> bool {
    // Keep the pre-release baseline namespace private without rewriting journals.
    plugin.starts_with('_') || plugin == "bingo.context"
}

fn is_empty_meta(m: &ProviderMetadata) -> bool {
    m.is_empty()
}

fn is_empty_map(m: &serde_json::Map<String, Value>) -> bool {
    m.is_empty()
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct Frame {
    pub seq: Seq,
    #[schemars(with = "String")]
    pub ts: Timestamp,
    pub session: SessionId,
    /// The client intent this frame answers or results from, when there is one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cause: Option<IntentId>,
    pub event: Event,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(
    remote = "Self",
    tag = "type",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum Event {
    SessionUpdated {
        summary: SessionSummary,
    },
    SessionClosed {
        reason: CloseReason,
    },

    TurnStarted {
        turn: TurnId,
        inputs: Vec<ItemId>,
        origin: TurnOrigin,
    },
    TurnRetrying {
        turn: TurnId,
        attempt: u32,
        max: u32,
        delay_ms: u64,
        dropped: Vec<ItemId>,
        reason: String,
    },
    TurnUsage {
        turn: TurnId,
        usage: Usage,
        context: ContextUsage,
    },
    /// Where the context stands outside a round: published when a segment
    /// opens and when the model changes, once the window is known. `None`
    /// says the window is no longer known.
    ContextChanged {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        context: Option<ContextUsage>,
    },
    /// Additive model spend, independent of context measurement and rounds.
    UsageCharged {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        turn: Option<TurnId>,
        plugin: String,
        usage: Usage,
    },
    TurnCompleted {
        turn: TurnId,
        status: TurnStatus,
        usage: Usage,
    },

    ItemStarted {
        item: Item,
    },
    /// Ephemeral. `ItemCompleted` is authoritative over every delta before it.
    ItemDelta {
        item: ItemId,
        n: u32,
        kind: DeltaKind,
        data: String,
    },
    ItemUpdated {
        item: Item,
    },
    ItemCompleted {
        item: Item,
    },

    QueueChanged {
        revision: u64,
        entries: Vec<QueueEntry>,
    },

    InteractionOpened {
        interaction: Interaction,
    },
    InteractionResolved {
        id: InteractionId,
        answer: Answer,
        by: ResolvedBy,
    },
    InteractionCancelled {
        id: InteractionId,
        reason: CancelReason,
    },

    IntentAck {
        intent: IntentId,
        outcome: IntentOutcome,
    },

    Compacted {
        generation: u64,
        boundary: ItemId,
        summary: ItemId,
        kept: Vec<ItemId>,
    },
    Rewound {
        generation: u64,
        to_turn: TurnId,
        dropped: Vec<ItemId>,
        files_restored: Vec<String>,
    },

    ConfigChanged {
        config: ConfigView,
    },
    CatalogChanged {
        kind: String,
    },

    /// Ephemeral. Transcript-worthy notices are `ItemBody::Notice`.
    Notice {
        level: Level,
        code: String,
        text: String,
    },
    /// A plugin-owned resource changed (roster, room, task…). The kernel does not enumerate these.
    Extension {
        plugin: String,
        kind: String,
        payload: Value,
    },
    /// Ephemeral. A plugin's live state (ADR-0013 §2): the latest payload per
    /// `(plugin, kind)` is the whole of it; `Null` removes it; none survives a resume.
    Invalidated {
        requests: Vec<String>,
    },
    Signal {
        #[serde(skip)]
        generation: Option<ItemId>,
        plugin: String,
        kind: String,
        payload: Value,
    },

    /// Ephemeral, transport only: this subscriber missed `from..=to`; re-read the journal.
    Lagged {
        from: Seq,
        to: Seq,
    },
    /// A payload this build cannot read, carried without interpretation.
    #[serde(skip)]
    #[schemars(!skip, untagged)]
    Unknown {
        #[schemars(flatten)]
        raw: UnknownRecord,
    },
}

impl Event {
    /// Durable events are journaled; the rest exist only on the live stream.
    pub fn is_durable(&self) -> bool {
        !matches!(
            self,
            Event::ItemDelta { .. }
                | Event::Notice { .. }
                | Event::Invalidated { .. }
                | Event::Signal { .. }
                | Event::Lagged { .. }
        )
    }

    /// The one rule for `SessionSummary::messages`: a message is counted where
    /// it is finished, so the fold, the store's append and a summary rebuilt
    /// from the whole journal reach the same number however many times the
    /// item was updated on the way there.
    pub fn completes_a_message(&self) -> bool {
        matches!(self, Event::ItemCompleted { item } if item.body.is_message())
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub enum DeltaKind {
    /// Append to assistant text.
    Text,
    /// Append to reasoning text.
    Reasoning,
    /// Replace the progress tail of a running tool call.
    Tail,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub enum Level {
    Info,
    Warn,
    Error,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum CloseReason {
    Client,
    Shutdown,
    Deleted,
    Error { message: String },
}

/// How a producer intends an utterance to be drawn.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub enum Voice {
    Said,
    Note,
    Quiet,
    Aside,
}

/// Who submitted the input, as every surface records it.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct Origin {
    /// Drawing family of the caller, independent of its attribution identifier.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub render_surface: Option<crate::render::SurfaceName>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub voice: Option<Voice>,
    pub surface: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub principal: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub conversation: Option<String>,
}

impl Origin {
    pub fn on(mut self, surface: crate::render::SurfaceName) -> Self {
        self.render_surface = Some(surface);
        self
    }

    pub fn voiced(mut self, voice: Voice) -> Self {
        self.voice = Some(voice);
        self
    }

    /// Model context, not a person's utterance or an agent's reply.
    pub fn is_context(&self) -> bool {
        self.surface.starts_with(CONTRIBUTOR_PREFIX)
    }

    pub fn surface(name: impl Into<String>) -> Self {
        Self {
            render_surface: None,
            voice: None,
            surface: name.into(),
            principal: None,
            conversation: None,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub enum TurnOrigin {
    /// A client submitted while the session was idle.
    Submit,
    /// Drained from the queue when the previous turn ended.
    Queue,
    /// Another session posted into this one.
    Peer,
    /// The kernel or a plugin opened the turn with no prose.
    Auto,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum TurnStatus {
    Completed,
    Failed { error: KernelError },
    Interrupted { reason: InterruptReason },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub enum InterruptReason {
    UserCancel,
    NewInput,
    Shutdown,
    Budget,
}

/// The one ruler for context: used tokens, the model window, and the point at
/// which compaction triggers.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ContextUsage {
    /// Approximate local request composition, independent of server anchoring.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parts: Option<[ContextPart; 4]>,
    /// Tokens the next request would carry, anchored on the server's last count.
    pub used: u64,
    /// The input side of the model's window: what is left once the output
    /// budget is reserved (ADR-0006).
    pub window: u64,
    /// `used` at which the older turns are summarised, equal to `window`
    /// where the kernel draws no line because the endpoint holds the context
    /// and cuts it itself (ADR-0055 §2).
    pub trigger: u64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ContextPart {
    pub name: ContextKind,
    pub tokens: u64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum ContextKind {
    System,
    Tools,
    Chat,
    Pictures,
}
impl std::fmt::Display for ContextKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::System => "system",
            Self::Tools => "tools",
            Self::Chat => "chat",
            Self::Pictures => "pictures",
        })
    }
}

impl ContextUsage {
    pub fn percent(&self) -> u64 {
        (self.used * 100).checked_div(self.window).unwrap_or(0)
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct Item {
    pub id: ItemId,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub turn: Option<TurnId>,
    #[serde(default)]
    pub round: u32,
    pub status: ItemStatus,
    #[schemars(with = "String")]
    pub started_at: Timestamp,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(with = "Option<String>")]
    pub completed_at: Option<Timestamp>,
    /// The client intent that produced this item, when one did (user items).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub intent: Option<IntentId>,
    pub body: ItemBody,
    #[serde(default, skip_serializing_if = "is_empty_map")]
    pub meta: serde_json::Map<String, Value>,
}

/// The meta key [`Item::external`] reads. A key rather than a field: it is
/// true of a handful of items and false of every other, and `meta` is where an
/// item's occasional facts already live.
const EXTERNAL: &str = "external";
// Like external, this is an occasional item fact, not a field on every item.
const HEADLINE: &str = "headline";

impl Item {
    pub fn headline(&self) -> Option<crate::Headline> {
        serde_json::from_value(self.meta.get(HEADLINE)?.clone()).ok()
    }

    pub fn set_headline(&mut self, headline: crate::Headline) {
        self.meta
            .insert(HEADLINE.into(), serde_json::json!(headline));
    }

    pub fn is_terminal(&self) -> bool {
        self.status.is_terminal()
    }

    /// Whether the session's own model neither asked for this nor is owed it:
    /// a call handed in through [`crate::host::HostApi::invoke`] is run,
    /// journaled and rendered like any other, but it belongs to the caller
    /// that handed it in. The provider fold skips it — replaying it would put
    /// words in a model's mouth that it never said.
    pub fn external(&self) -> bool {
        self.meta.get(EXTERNAL).and_then(Value::as_bool) == Some(true)
    }

    pub fn mark_external(&mut self) {
        self.meta.insert(EXTERNAL.into(), Value::Bool(true));
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub enum ItemStatus {
    Pending,
    Running,
    Completed,
    Failed,
    Interrupted,
}

impl ItemStatus {
    pub fn is_terminal(self) -> bool {
        matches!(
            self,
            ItemStatus::Completed | ItemStatus::Failed | ItemStatus::Interrupted
        )
    }

    /// Whether a tool call came back wrong: this status, or an output the
    /// tool marked as an error. One question asked in one place, so the
    /// verdict a run prints and the bullet a screen draws can never disagree.
    pub fn failed(self, output: Option<&ToolOutput>) -> bool {
        self == ItemStatus::Failed || output.is_some_and(|output| output.is_error)
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(
    remote = "Self",
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum ItemBody {
    User {
        parts: Vec<ContentPart>,
        origin: Origin,
    },
    Assistant {
        text: String,
    },
    Reasoning {
        text: String,
        #[serde(default, skip_serializing_if = "is_empty_meta")]
        provider_metadata: ProviderMetadata,
    },
    ToolCall {
        /// The policy that answered without opening an ask.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        allowed_by: Option<Box<crate::policy::Reason>>,
        /// When the call's own ask opened. Its wait runs until `began_at`, or
        /// until `completed_at` for a call that never ran.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[schemars(with = "Option<String>")]
        asked_at: Option<Timestamp>,
        /// When the call began running.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[schemars(with = "Option<String>")]
        began_at: Option<Timestamp>,
        call_id: String,
        name: String,
        input: Value,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        output: Option<Box<ToolOutput>>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        progress: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        duration_ms: Option<u64>,
        #[serde(default, skip_serializing_if = "std::ops::Not::not")]
        quiet: bool,
    },
    /// A shell line a person ran themselves (`!<command>`, ADR-0008 §5): no
    /// turn was spent on it and no model asked for it. `exit` is the code the
    /// command came to; `None` says it never reached one — its timeout or an
    /// interrupt ended it, and `output` carries the line that says which.
    Shell {
        command: String,
        output: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        exit: Option<i32>,
        cwd: std::path::PathBuf,
    },
    /// A long-running non-turn operation (login, reconnect, team start).
    Action {
        name: String,
        #[serde(default)]
        args: Value,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        result: Option<Value>,
    },
    Compaction {
        summary: String,
        replaced: u32,
        before: u64,
        after: u64,
        duration_ms: u64,
    },
    Rewind {
        to_turn: TurnId,
        dropped: u32,
    },
    Interruption {
        marker: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[schemars(with = "Option<String>")]
        asked_at: Option<Timestamp>,
    },
    Notice {
        level: Level,
        code: String,
        text: String,
    },
    QuestionAnswer {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[schemars(with = "Option<String>")]
        asked_at: Option<Timestamp>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        item: Option<ItemId>,
        interaction: InteractionId,
        question: String,
        answer: String,
    },
    PermissionReceipt {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[schemars(with = "Option<String>")]
        asked_at: Option<Timestamp>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        item: Option<ItemId>,
        interaction: InteractionId,
        tool: String,
        decision: DecisionKind,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        feedback: Option<String>,
    },
    Asset {
        asset: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        label: Option<String>,
    },
    /// A payload this build cannot read, carried without interpretation.
    #[serde(skip)]
    #[schemars(!skip, untagged)]
    Unknown {
        #[schemars(flatten)]
        raw: UnknownRecord,
    },
}

impl ItemBody {
    /// What a person reads as a message: their own prose and the model's
    /// answers. Tool calls, receipts, notices and contributor context are work
    /// around messages; counting them would make a busy session look chatty.
    pub fn is_message(&self) -> bool {
        match self {
            ItemBody::User { origin, .. } => !origin.is_context(),
            ItemBody::Assistant { .. } => true,
            _ => false,
        }
    }

    /// The words of a command's receipt: an informational notice, which the
    /// command's ack also carries for a client that draws no transcript. A
    /// warning or an error is a diagnostic, never a command's answer.
    pub fn receipt(&self) -> Option<&str> {
        match self {
            ItemBody::Notice {
                level: Level::Info,
                text,
                ..
            } => Some(text),
            _ => None,
        }
    }
}

/// A shell line as a model reads it ([`ItemBody::Shell`]): the line under the
/// prompt it was typed at, what it wrote in a fence of its own, and the code
/// it came to when that was not a clean exit. A command that reached no code
/// says so in its own output and adds nothing here.
///
/// One rendering, written down once: the kernel's fold puts these words in
/// the next request, and a surface reporting to a host quotes the same ones.
pub fn shell_note(command: &str, output: &str, exit: Option<i32>) -> String {
    let mut out = format!("$ {command}");
    let body = output.trim_end_matches('\n');
    if !body.is_empty() {
        out.push_str(&format!("\n```\n{body}\n```"));
    }
    if let Some(code) = exit.filter(|code| *code != 0) {
        out.push_str(&format!("\n[exit {code}]"));
    }
    out
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub enum DecisionKind {
    AllowChanged,
    AllowChangedSession,
    Allow,
    AllowSession,
    Deny,
}

/// What a tool returned: the parts the model sees, plus what a person sees
/// instead when the tool has something better than the text (ADR-0013 §2).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ToolOutput {
    pub parts: Vec<ContentPart>,
    #[serde(default, skip_serializing_if = "is_false")]
    pub is_error: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub display: Option<View>,
    /// What the call came to in one line, for the step that shows it
    /// (ADR-0095 §9). The model never reads it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub upshot: Option<Upshot>,
}

impl ToolOutput {
    pub fn text(text: impl Into<String>) -> Self {
        Self {
            parts: vec![ContentPart::text(text)],
            is_error: false,
            display: None,
            upshot: None,
        }
    }

    pub fn error(text: impl Into<String>) -> Self {
        Self {
            parts: vec![ContentPart::text(text)],
            is_error: true,
            display: None,
            upshot: None,
        }
    }

    pub fn with_upshot(mut self, upshot: Upshot) -> Self {
        self.upshot = Some(upshot);
        self
    }

    /// What the call came to: the line the tool wrote, else the counts of the
    /// change it showed. The diff is the fact and its counts are read off it,
    /// never written beside it.
    pub fn outcome(&self) -> Option<Upshot> {
        self.upshot.clone().or_else(|| match &self.display {
            Some(View::Diff { unified }) => Some(Upshot::of_diff(unified)),
            _ => None,
        })
    }
}

impl Upshot {
    /// The lines a unified diff adds and removes, counted inside its hunks.
    pub fn of_diff(unified: &str) -> Self {
        let (added, removed) = hunks::counts(unified);
        Self::Diff { added, removed }
    }

    pub fn plain(text: impl Into<String>) -> Self {
        Self::Words {
            text: text.into(),
            tone: Tone::Neutral,
        }
    }

    pub fn good(text: impl Into<String>) -> Self {
        Self::Words {
            text: text.into(),
            tone: Tone::Good,
        }
    }

    pub fn bad(text: impl Into<String>) -> Self {
        Self::Words {
            text: text.into(),
            tone: Tone::Bad,
        }
    }
}

/// A call's result in one line: `+1 −1`, `3 passed`, `exit 1`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum Upshot {
    /// Lines a change added and removed; counts add up across a turn.
    Diff { added: u32, removed: u32 },
    /// A few words, and whether they are good or bad news.
    Words { text: String, tone: Tone },
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct QueueEntry {
    pub intent: IntentId,
    pub position: u32,
    pub preview: String,
    pub steerable: bool,
    pub origin: Origin,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum IntentOutcome {
    TurnStarted { turn: TurnId },
    Queued { position: u32 },
    Applied { result: Value },
    Rejected { error: KernelError },
}

/// Where a child hangs in the tree: its parent, and the tool call that
/// spawned it when one did (ADR-0011 §3).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ParentLink {
    pub session: SessionId,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub item: Option<ItemId>,
}

/// What a session does with what it is told (ADR-0011 §1).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub enum Driver {
    /// A model answers: every prose input opens a turn.
    #[default]
    Model,
    /// Nothing answers: every input is recorded, and the journal is the point.
    Log,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct SessionSummary {
    pub id: SessionId,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub key: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    pub cwd: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parent: Option<ParentLink>,
    #[serde(default)]
    pub driver: Driver,
    #[serde(flatten)]
    pub choice: SessionModel,
    /// What the session was opened with and a resume must give it back:
    /// the prompt appended to the kernel's own, and the tool set it is held to.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub system_extra: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tools: Option<Vec<String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub provider: Option<String>,
    #[schemars(with = "String")]
    pub created_at: Timestamp,
    #[schemars(with = "String")]
    pub updated_at: Timestamp,
    #[serde(default)]
    pub usage: Usage,
    /// A turn is running. A summary travels without the turn it describes —
    /// a host listing carries no `SessionState` — so the kernel stamps this
    /// on every summary it publishes and nothing else writes it. A client
    /// that holds the state derives the same fact with `SessionState::busy`.
    #[serde(default)]
    pub busy: bool,
    /// An interaction is open; stamped only by the session actor.
    #[serde(default)]
    pub asking: bool,
    #[serde(flatten)]
    pub history: SessionHistory,
}

// Group the choice in Rust while keeping its fields flat on the wire.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct SessionModel {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub model_scope: Option<ChoiceScope>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub effort: Option<crate::Effort>,
}

impl std::ops::Deref for SessionSummary {
    type Target = SessionModel;
    fn deref(&self) -> &Self::Target {
        &self.choice
    }
}
impl std::ops::DerefMut for SessionSummary {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.choice
    }
}

/// Whether the chosen model applies only here or is the default for new sessions.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub enum ChoiceScope {
    Session,
    Default,
}

// Journal facts used to find and describe a conversation without replaying it.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct SessionHistory {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub first_ask: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub forked_from: Option<ForkLink>,
    /// How many messages have been said in this session, by
    /// [`Event::completes_a_message`]. `None` is "never counted" — a summary
    /// written before the field existed — and must be shown as nothing rather
    /// than as a `0` it does not mean.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub messages: Option<u64>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ForkLink {
    pub session: SessionId,
    pub turn: Option<TurnId>,
}

/// Configuration as a client sees it: the kernel's keys and each plugin's claimed slice.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ConfigView {
    #[serde(default)]
    pub kernel: Value,
    #[serde(default)]
    pub plugins: BTreeMap<String, Value>,
}

impl ConfigView {
    /// Where the kernel names the session's policy: `kernel.policy` holds its
    /// id, and `plugins[id]` what it says of itself (ADR-0009 §5). Both are
    /// the kernel's to write; no hook may move them.
    pub const POLICY: &str = "policy";

    /// The session's policy and its own description, and no other plugin's
    /// slice, however it sorts or whatever it claims to be.
    pub fn policy(&self) -> Option<(&str, &Value)> {
        let id = self.kernel.get(Self::POLICY)?.as_str()?;
        self.plugins
            .get_key_value(id)
            .map(|(id, value)| (id.as_str(), value))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn frame_summary_reads_old_records_and_round_trips_choice_facts() {
        let mut value = serde_json::from_str::<serde_json::Value>(include_str!(
            "../fixtures/record/compat/summary-before-asking.json"
        ))
        .unwrap()["summary"]
            .clone();
        value["effort"] = serde_json::json!("medium");
        value["modelScope"] = serde_json::json!("session");
        let summary: SessionSummary = serde_json::from_value(value).unwrap();
        assert_eq!(summary.effort, Some(crate::Effort::Medium));
        assert_eq!(summary.model_scope, Some(ChoiceScope::Session));
        let encoded = serde_json::to_value(summary).unwrap();
        assert_eq!(encoded["modelScope"], "session");
        assert_eq!(encoded["effort"], "medium");
        let old: SessionSummary = serde_json::from_value(
            serde_json::from_str::<serde_json::Value>(include_str!(
                "../fixtures/record/compat/summary-before-asking.json"
            ))
            .unwrap()["summary"]
                .clone(),
        )
        .unwrap();
        assert_eq!(old.model_scope, None);
        assert_eq!(old.effort, None);
    }

    #[test]
    fn a_summary_before_asking_defaults_to_false() {
        let value: serde_json::Value = serde_json::from_str(include_str!(
            "../fixtures/record/compat/summary-before-asking.json"
        ))
        .unwrap();
        let summary: SessionSummary = serde_json::from_value(value["summary"].clone()).unwrap();
        assert!(!summary.asking);
    }

    fn ts() -> Timestamp {
        Timestamp::from_second(1_700_000_000).unwrap()
    }

    fn item(id: &str, body: ItemBody) -> Item {
        Item {
            id: ItemId::from_raw(id),
            turn: Some(TurnId::from_raw("trn_1")),
            round: 0,
            status: ItemStatus::Completed,
            started_at: ts(),
            completed_at: Some(ts()),
            intent: None,
            body,
            meta: Default::default(),
        }
    }

    #[test]
    fn contributor_context_does_not_count_as_a_conversation_message() {
        for (surface, counted) in [
            ("tui", true),
            ("peer", true),
            ("contributor:inventory", false),
            ("contributor-extra", true),
        ] {
            let event = Event::ItemCompleted {
                item: item(
                    "itm_context",
                    ItemBody::User {
                        parts: vec![ContentPart::text("current state")],
                        origin: Origin::surface(surface),
                    },
                ),
            };
            let restored: Event =
                serde_json::from_value(serde_json::to_value(&event).unwrap()).unwrap();
            assert_eq!(event.completes_a_message(), counted, "{surface}");
            assert_eq!(restored.completes_a_message(), counted, "{surface}");
        }
    }

    #[test]
    fn only_deltas_notices_and_lag_are_ephemeral() {
        assert!(
            !Event::ItemDelta {
                item: ItemId::from_raw("i"),
                n: 0,
                kind: DeltaKind::Text,
                data: String::new()
            }
            .is_durable()
        );
        assert!(
            !Event::Notice {
                level: Level::Info,
                code: "X".into(),
                text: String::new()
            }
            .is_durable()
        );
        assert!(
            !Event::Lagged {
                from: Seq(1),
                to: Seq(2)
            }
            .is_durable()
        );
        assert!(
            Event::SessionClosed {
                reason: CloseReason::Client
            }
            .is_durable()
        );
    }

    fn option(id: &str, role: Option<AnswerRole>) -> QuestionOption {
        QuestionOption {
            id: id.into(),
            label: id.into(),
            description: None,
            role,
            preview: None,
        }
    }

    fn asked(options: Vec<QuestionOption>) -> Question {
        Question {
            question: "may it?".into(),
            header: None,
            options,
            free_text: false,
            multi: false,
        }
    }

    fn question(options: Vec<QuestionOption>) -> InteractionKind {
        InteractionKind::Question(asked(options))
    }

    #[test]
    fn a_question_names_the_option_each_role_is_answered_with() {
        let kind = question(vec![
            option("yes", Some(AnswerRole::Allowing)),
            option("always", None),
            option("no", Some(AnswerRole::Refusing)),
        ]);
        assert_eq!(
            kind.answer_for(AnswerRole::Allowing),
            Some(Answer::Choice {
                ids: vec!["yes".into()],
                other: None,
            })
        );
        assert_eq!(
            kind.answer_for(AnswerRole::Refusing),
            Some(Answer::Choice {
                ids: vec!["no".into()],
                other: None,
            })
        );
    }

    #[test]
    fn a_question_nobody_marked_is_a_persons_alone() {
        let unmarked = question(vec![option("yes", None)]);
        assert_eq!(unmarked.answer_for(AnswerRole::Allowing), None);
        assert_eq!(unmarked.answer_for(AnswerRole::Refusing), None);
        let confirm = InteractionKind::Confirm {
            title: "t".into(),
            detail: "d".into(),
        };
        assert_eq!(confirm.answer_for(AnswerRole::Allowing), None);
    }

    #[test]
    fn a_form_is_answered_by_every_questions_role_option() {
        let kind = InteractionKind::Form {
            title: None,
            questions: vec![
                asked(vec![
                    option("yes", Some(AnswerRole::Allowing)),
                    option("no", Some(AnswerRole::Refusing)),
                ]),
                asked(vec![
                    option("go", Some(AnswerRole::Allowing)),
                    option("stop", Some(AnswerRole::Refusing)),
                ]),
            ],
        };
        assert_eq!(
            kind.answer_for(AnswerRole::Allowing),
            Some(Answer::Form {
                answers: vec![
                    Answer::Choice {
                        ids: vec!["yes".into()],
                        other: None,
                    },
                    Answer::Choice {
                        ids: vec!["go".into()],
                        other: None,
                    },
                ]
            })
        );
        assert_eq!(
            kind.answer_for(AnswerRole::Refusing),
            Some(Answer::Form {
                answers: vec![
                    Answer::Choice {
                        ids: vec!["no".into()],
                        other: None,
                    },
                    Answer::Choice {
                        ids: vec!["stop".into()],
                        other: None,
                    },
                ]
            })
        );
        assert_eq!(Answer::Form { answers: vec![] }.spec(), AnswerSpec::Form);
    }

    #[test]
    fn a_form_one_of_whose_questions_is_unmarked_is_a_persons_alone() {
        let kind = InteractionKind::Form {
            title: None,
            questions: vec![
                asked(vec![option("yes", Some(AnswerRole::Allowing))]),
                asked(vec![option("go", None)]),
            ],
        };
        assert_eq!(kind.answer_for(AnswerRole::Allowing), None);
        assert_eq!(kind.answer_for(AnswerRole::Refusing), None);
    }

    /// The lone question holds the same struct a form's questions are, so
    /// every frame written before it was extracted still reads (M53).
    #[test]
    fn a_bare_question_on_the_wire_is_the_shape_it_always_was() {
        let json = serde_json::json!({
            "kind": "question",
            "question": "Which authentication method?",
            "header": "Auth method",
            "options": [{ "id": "0", "label": "OAuth" }],
            "freeText": true,
            "multi": false
        });
        let kind: InteractionKind = serde_json::from_value(json.clone()).expect("an old question");
        assert_eq!(
            kind,
            InteractionKind::Question(Question {
                question: "Which authentication method?".into(),
                header: Some("Auth method".into()),
                options: vec![QuestionOption {
                    id: "0".into(),
                    label: "OAuth".into(),
                    description: None,
                    role: None,
                    preview: None,
                }],
                free_text: true,
                multi: false,
            })
        );
        assert_eq!(serde_json::to_value(&kind).expect("json"), json);
    }

    /// A choice carries the words a person typed beside their ticks, and a
    /// frame written before there was a field for them reads as the ticks
    /// alone (M59).
    #[test]
    fn a_choice_carries_the_words_typed_beside_the_ticks() {
        let bare = serde_json::json!({ "kind": "choice", "ids": ["0", "1"] });
        assert_eq!(
            serde_json::from_value::<Answer>(bare.clone()).expect("an old choice"),
            Answer::Choice {
                ids: vec!["0".into(), "1".into()],
                other: None,
            }
        );
        assert_eq!(
            serde_json::to_value(Answer::Choice {
                ids: vec!["0".into(), "1".into()],
                other: None,
            })
            .expect("json"),
            bare,
            "a choice with no words of its own is on the wire as it always was"
        );
        let both = serde_json::json!({
            "kind": "choice",
            "ids": ["0", "1"],
            "other": "and a place of my own"
        });
        let answer = Answer::Choice {
            ids: vec!["0".into(), "1".into()],
            other: Some("and a place of my own".into()),
        };
        assert_eq!(
            serde_json::from_value::<Answer>(both.clone()).expect("a choice with words"),
            answer
        );
        assert_eq!(serde_json::to_value(&answer).expect("json"), both);
        assert_eq!(answer.spec(), AnswerSpec::Choice);
    }

    fn asking(kind: InteractionKind, answers: Vec<AnswerSpec>) -> Interaction {
        Interaction {
            id: InteractionId::from_raw("int_1"),
            session: SessionId::from_raw("ses_1"),
            turn: None,
            item: None,
            opened_at: ts(),
            guard_until: None,
            expires_at: None,
            kind,
            answers,
        }
    }

    fn permission(session_scope: Option<&str>) -> InteractionKind {
        InteractionKind::Permission {
            tool: "Read".into(),
            summary: "Read Cargo.toml".into(),
            preview: None,
            session_scope: session_scope.map(str::to_string),
            caution: None,
            input: Default::default(),
        }
    }

    fn answers_of(interaction: &Interaction) -> Vec<Answer> {
        interaction
            .rungs()
            .into_iter()
            .map(|rung| rung.answer)
            .collect()
    }

    /// The widest first, and only the ones the kernel said it would take.
    #[test]
    fn a_permission_offers_the_answers_it_says_it_takes() {
        let all = asking(
            permission(Some("Read(src/)")),
            vec![
                AnswerSpec::AllowOnce,
                AnswerSpec::AllowSession,
                AnswerSpec::Deny,
            ],
        );
        assert_eq!(
            answers_of(&all),
            vec![
                Answer::AllowOnce,
                Answer::AllowSession {
                    scope: "Read(src/)".into()
                },
                Answer::Deny { feedback: None },
            ]
        );
        assert_eq!(
            all.rungs()[1].label,
            "Allow Read(src/) for this session",
            "the scope is named in the words of the rung that installs it"
        );

        let narrow = asking(permission(None), vec![AnswerSpec::Deny]);
        assert_eq!(answers_of(&narrow), vec![Answer::Deny { feedback: None }]);
    }

    /// A session rule needs a scope to be a rule; without one the rung would
    /// promise something no policy could hold.
    #[test]
    fn a_permission_with_no_scope_offers_no_session_rule() {
        let scopeless = asking(
            permission(None),
            vec![
                AnswerSpec::AllowOnce,
                AnswerSpec::AllowSession,
                AnswerSpec::Deny,
            ],
        );
        assert_eq!(
            answers_of(&scopeless),
            vec![Answer::AllowOnce, Answer::Deny { feedback: None }]
        );
    }

    #[test]
    fn a_question_offers_one_rung_per_option_in_the_askers_own_words() {
        let asked = asking(
            InteractionKind::Question(Question {
                question: "which one?".into(),
                header: None,
                options: vec![option("a", None), option("b", None)],
                free_text: true,
                multi: false,
            }),
            vec![AnswerSpec::Choice, AnswerSpec::Text, AnswerSpec::Cancel],
        );
        let rungs = asked.rungs();
        assert_eq!(
            rungs.iter().map(|r| r.label.clone()).collect::<Vec<_>>(),
            vec!["a", "b"],
            "words of one's own are a door, not a rung, and cancel is not offered"
        );
        assert_eq!(
            rungs[0].answer,
            Answer::Choice {
                ids: vec!["a".into()],
                other: None,
            }
        );

        let words_only = asking(
            InteractionKind::Question(Question {
                question: "what shall I call it?".into(),
                header: None,
                options: vec![option("a", None)],
                free_text: true,
                multi: false,
            }),
            vec![AnswerSpec::Text, AnswerSpec::Cancel],
        );
        assert!(
            words_only.rungs().is_empty(),
            "an option the kernel would refuse is not offered"
        );
    }

    #[test]
    fn a_confirm_offers_both_ways_out_and_a_form_offers_no_rung() {
        let confirm = asking(
            InteractionKind::Confirm {
                title: "Delete it?".into(),
                detail: "for good".into(),
            },
            vec![AnswerSpec::Confirm, AnswerSpec::Cancel],
        );
        assert_eq!(answers_of(&confirm), vec![Answer::Confirm, Answer::Cancel]);

        let one_way = asking(
            InteractionKind::Confirm {
                title: "Delete it?".into(),
                detail: "for good".into(),
            },
            vec![AnswerSpec::Confirm],
        );
        assert_eq!(answers_of(&one_way), vec![Answer::Confirm]);

        let form = asking(
            InteractionKind::Form {
                title: None,
                questions: Vec::new(),
            },
            vec![AnswerSpec::Form, AnswerSpec::Cancel],
        );
        assert!(
            form.rungs().is_empty(),
            "a form is answered question by question"
        );
    }

    /// A browser flow finishes on its own; the one rung is the way out, and
    /// the credential a paste flow wants is words, not a rung.
    #[test]
    fn a_login_offers_the_way_out() {
        let paste = asking(
            InteractionKind::Login {
                provider: "anthropic".into(),
                flow: LoginFlow::Paste,
            },
            vec![AnswerSpec::Text, AnswerSpec::Cancel],
        );
        assert_eq!(answers_of(&paste), vec![Answer::Cancel]);
    }

    /// A call is wrong when its status says so or when its output does; a
    /// completed call with a clean output is the only verdict that is right.
    #[test]
    fn a_call_fails_on_its_status_or_on_the_output_it_marked() {
        let clean = ToolOutput::text("ok");
        let wrong = ToolOutput::error("no");
        assert!(!ItemStatus::Completed.failed(None));
        assert!(!ItemStatus::Completed.failed(Some(&clean)));
        assert!(ItemStatus::Completed.failed(Some(&wrong)));
        assert!(ItemStatus::Failed.failed(None));
        assert!(ItemStatus::Failed.failed(Some(&clean)));
        assert!(!ItemStatus::Interrupted.failed(Some(&clean)));
    }

    /// A change's counts come off its diff, so the tool writes them once;
    /// a line the tool wrote wins over them.
    #[test]
    fn a_calls_outcome_is_its_own_line_else_its_diffs_counts() {
        let unified = "--- a.rs\n+++ a.rs\n@@ -1,2 +1,3 @@\n one\n-two\n+too\n+three\n";
        let mut changed = ToolOutput::text("Edited a.rs");
        changed.display = Some(View::Diff {
            unified: unified.into(),
        });
        assert_eq!(
            changed.outcome(),
            Some(Upshot::Diff {
                added: 2,
                removed: 1
            })
        );
        let said = ToolOutput::text("ok").with_upshot(Upshot::good("3 passed"));
        assert_eq!(said.outcome(), Some(Upshot::good("3 passed")));
        assert_eq!(ToolOutput::text("plain").outcome(), None);
        let wire = serde_json::to_value(Upshot::bad("exit 1")).expect("an upshot");
        assert_eq!(
            wire,
            serde_json::json!({ "kind": "words", "text": "exit 1", "tone": "bad" })
        );
    }

    #[test]
    fn an_unmarked_option_is_on_the_wire_as_it_was() {
        let json = serde_json::to_value(option("yes", None)).expect("an option");
        assert_eq!(json, serde_json::json!({ "id": "yes", "label": "yes" }));
        assert_eq!(
            serde_json::to_value(option("yes", Some(AnswerRole::Allowing))).expect("an option"),
            serde_json::json!({ "id": "yes", "label": "yes", "role": "allowing" })
        );
    }
}

#[cfg(test)]
mod context_parts_contract {
    use super::*;
    #[test]
    fn parts_are_additive_and_round_trip_the_fixture() {
        let old = serde_json::json!({"used":42,"window":100,"trigger":90});
        let usage: ContextUsage = serde_json::from_value(old.clone()).unwrap();
        assert!(usage.parts.is_none());
        assert_eq!(serde_json::to_value(usage).unwrap(), old);
        let wire: serde_json::Value =
            serde_json::from_str(include_str!("../fixtures/context-parts.json")).unwrap();
        let usage: ContextUsage = serde_json::from_value(wire.clone()).unwrap();
        assert_eq!(
            usage.parts.unwrap().iter().map(|p| p.tokens).sum::<u64>(),
            1605
        );
        assert_ne!(
            usage.parts.unwrap().iter().map(|p| p.tokens).sum::<u64>(),
            usage.used
        );
        assert_eq!(serde_json::to_value(usage).unwrap(), wire);
    }
}
