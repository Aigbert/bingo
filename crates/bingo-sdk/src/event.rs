//! The one vocabulary that crosses kernel → client. A `Frame` is one line of a
//! session's journal; `Event` is everything that can happen in a session; an
//! `Item` is one unit in transcript order. Deltas, notices and lag markers are
//! ephemeral (never journaled); everything else is durable.

mod interaction;
mod raw;
mod record;

pub use interaction::{
    Activation, Answer, AnswerRole, AnswerSpec, CancelReason, Interaction, InteractionKind,
    LoginFlow, Preview, Question, QuestionOption, ResolvedBy, Rung,
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
use crate::view::View;

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
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub voice: Option<Voice>,
    pub surface: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub principal: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub conversation: Option<String>,
}

impl Origin {
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

impl Item {
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
        call_id: String,
        name: String,
        input: Value,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        output: Option<ToolOutput>,
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
    },
    Notice {
        level: Level,
        code: String,
        text: String,
    },
    QuestionAnswer {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        item: Option<ItemId>,
        interaction: InteractionId,
        question: String,
        answer: String,
    },
    PermissionReceipt {
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
}

impl ToolOutput {
    pub fn text(text: impl Into<String>) -> Self {
        Self {
            parts: vec![ContentPart::text(text)],
            is_error: false,
            display: None,
        }
    }

    pub fn error(text: impl Into<String>) -> Self {
        Self {
            parts: vec![ContentPart::text(text)],
            is_error: true,
            display: None,
        }
    }
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
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
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
    /// How many messages have been said in this session, by
    /// [`Event::completes_a_message`]. `None` is "never counted" — a summary
    /// written before the field existed — and must be shown as nothing rather
    /// than as a `0` it does not mean.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub messages: Option<u64>,
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

#[cfg(test)]
mod tests {
    use super::*;

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

    fn frame(seq: u64, event: Event) -> Frame {
        Frame {
            seq: Seq(seq),
            ts: ts(),
            session: SessionId::from_raw("ses_1"),
            cause: None,
            event,
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

    /// One representative frame per event variant, pinned as JSON. Changing a
    /// snapshot is a wire change and deserves an ADR line.
    #[test]
    fn every_event_variant_has_a_pinned_wire_form() {
        let interaction = Interaction {
            id: InteractionId::from_raw("int_1"),
            session: SessionId::from_raw("ses_1"),
            turn: Some(TurnId::from_raw("trn_1")),
            item: Some(ItemId::from_raw("itm_3")),
            opened_at: ts(),
            guard_until: None,
            expires_at: None,
            kind: InteractionKind::Permission {
                tool: "Edit".into(),
                summary: "Edit src/lib.rs".into(),
                preview: Some(Preview::Diff {
                    unified: "--- a\n+++ b\n".into(),
                }),
                session_scope: Some("Edit(src/)".into()),
                caution: None,
                input: Default::default(),
            },
            answers: vec![
                AnswerSpec::AllowOnce,
                AnswerSpec::AllowSession,
                AnswerSpec::Deny,
            ],
        };
        let summary = SessionSummary {
            id: SessionId::from_raw("ses_1"),
            key: None,
            title: Some("hello".into()),
            cwd: "/tmp/p".into(),
            parent: None,
            driver: Driver::Model,
            model: Some("fake-1".into()),
            system_extra: None,
            tools: None,
            provider: Some("fake".into()),
            created_at: ts(),
            updated_at: ts(),
            usage: Usage::default(),
            busy: false,
            asking: false,
            messages: None,
        };
        let frames = vec![
            frame(
                1,
                Event::SessionUpdated {
                    summary: summary.clone(),
                },
            ),
            frame(
                2,
                Event::TurnStarted {
                    turn: TurnId::from_raw("trn_1"),
                    inputs: vec![ItemId::from_raw("itm_1")],
                    origin: TurnOrigin::Submit,
                },
            ),
            frame(
                3,
                Event::ItemCompleted {
                    item: item(
                        "itm_1",
                        ItemBody::User {
                            parts: vec![ContentPart::text("run tests")],
                            origin: Origin::surface("tui"),
                        },
                    ),
                },
            ),
            frame(
                4,
                Event::IntentAck {
                    intent: IntentId::from_raw("req_1"),
                    outcome: IntentOutcome::TurnStarted {
                        turn: TurnId::from_raw("trn_1"),
                    },
                },
            ),
            frame(
                5,
                Event::ItemStarted {
                    item: Item {
                        status: ItemStatus::Running,
                        completed_at: None,
                        ..item(
                            "itm_2",
                            ItemBody::Assistant {
                                text: String::new(),
                            },
                        )
                    },
                },
            ),
            frame(
                6,
                Event::ItemDelta {
                    item: ItemId::from_raw("itm_2"),
                    n: 0,
                    kind: DeltaKind::Text,
                    data: "Sure".into(),
                },
            ),
            frame(
                7,
                Event::ItemCompleted {
                    item: item(
                        "itm_2",
                        ItemBody::Assistant {
                            text: "Sure".into(),
                        },
                    ),
                },
            ),
            frame(
                8,
                Event::ItemStarted {
                    item: Item {
                        status: ItemStatus::Pending,
                        completed_at: None,
                        ..item(
                            "itm_3",
                            ItemBody::ToolCall {
                                call_id: "call_1".into(),
                                name: "Edit".into(),
                                input: serde_json::json!({"file_path": "src/lib.rs"}),
                                output: None,
                                progress: None,
                                duration_ms: None,
                                quiet: false,
                            },
                        )
                    },
                },
            ),
            frame(
                9,
                Event::InteractionOpened {
                    interaction: interaction.clone(),
                },
            ),
            frame(
                10,
                Event::InteractionResolved {
                    id: InteractionId::from_raw("int_1"),
                    answer: Answer::AllowOnce,
                    by: ResolvedBy::Client {
                        name: "gui-2".into(),
                        surface: "gui".into(),
                    },
                },
            ),
            frame(
                11,
                Event::InteractionCancelled {
                    id: InteractionId::from_raw("int_2"),
                    reason: CancelReason::TurnEnded,
                },
            ),
            frame(
                12,
                Event::ItemUpdated {
                    item: Item {
                        status: ItemStatus::Running,
                        completed_at: None,
                        ..item(
                            "itm_3",
                            ItemBody::ToolCall {
                                call_id: "call_1".into(),
                                name: "Edit".into(),
                                input: serde_json::json!({}),
                                output: None,
                                progress: Some("writing…".into()),
                                duration_ms: None,
                                quiet: false,
                            },
                        )
                    },
                },
            ),
            frame(
                13,
                Event::TurnUsage {
                    turn: TurnId::from_raw("trn_1"),
                    usage: Usage {
                        input_tokens: 10,
                        output_tokens: 4,
                        ..Default::default()
                    },
                    context: ContextUsage {
                        used: 14,
                        window: 200_000,
                        trigger: 180_000,
                    },
                },
            ),
            frame(
                14,
                Event::TurnRetrying {
                    turn: TurnId::from_raw("trn_1"),
                    attempt: 1,
                    max: 10,
                    delay_ms: 500,
                    dropped: vec![ItemId::from_raw("itm_4")],
                    reason: "server error 503".into(),
                },
            ),
            frame(
                15,
                Event::QueueChanged {
                    revision: 1,
                    entries: vec![QueueEntry {
                        intent: IntentId::from_raw("req_2"),
                        position: 0,
                        preview: "also fix the docs".into(),
                        steerable: true,
                        origin: Origin::surface("tui"),
                    }],
                },
            ),
            frame(
                16,
                Event::Compacted {
                    generation: 1,
                    boundary: ItemId::from_raw("itm_2"),
                    summary: ItemId::from_raw("itm_9"),
                    kept: vec![ItemId::from_raw("itm_1")],
                },
            ),
            frame(
                17,
                Event::Rewound {
                    generation: 2,
                    to_turn: TurnId::from_raw("trn_1"),
                    dropped: vec![ItemId::from_raw("itm_3")],
                    files_restored: vec!["src/lib.rs".into()],
                },
            ),
            frame(
                18,
                Event::ConfigChanged {
                    config: ConfigView::default(),
                },
            ),
            frame(
                19,
                Event::CatalogChanged {
                    kind: "models".into(),
                },
            ),
            frame(
                20,
                Event::Notice {
                    level: Level::Warn,
                    code: "COUNT_TOKENS_UNAVAILABLE".into(),
                    text: "estimating".into(),
                },
            ),
            frame(
                21,
                Event::Extension {
                    plugin: "bingo.tasks".into(),
                    kind: "task.changed".into(),
                    payload: serde_json::json!({"id": 1}),
                },
            ),
            frame(
                22,
                Event::Lagged {
                    from: Seq(3),
                    to: Seq(9),
                },
            ),
            frame(
                25,
                Event::Signal {
                    plugin: "bingo.demo.ui".into(),
                    kind: "progress".into(),
                    payload: serde_json::json!({"kind": "progress", "value": 3, "total": 10}),
                },
            ),
            frame(
                23,
                Event::TurnCompleted {
                    turn: TurnId::from_raw("trn_1"),
                    status: TurnStatus::Completed,
                    usage: Usage::default(),
                },
            ),
            frame(
                24,
                Event::SessionClosed {
                    reason: CloseReason::Client,
                },
            ),
            frame(
                26,
                Event::InteractionOpened {
                    interaction: Interaction {
                        id: InteractionId::from_raw("int_3"),
                        kind: InteractionKind::Form {
                            title: None,
                            questions: vec![
                                Question {
                                    question: "Which authentication method?".into(),
                                    header: Some("Auth method".into()),
                                    options: vec![QuestionOption {
                                        id: "0".into(),
                                        label: "OAuth".into(),
                                        description: Some("Redirect to the provider".into()),
                                        role: None,
                                        preview: Some("GET /authorize\n302 -> /callback".into()),
                                    }],
                                    free_text: true,
                                    multi: false,
                                },
                                Question {
                                    question: "Which targets?".into(),
                                    header: Some("Targets".into()),
                                    options: vec![QuestionOption {
                                        id: "0".into(),
                                        label: "linux".into(),
                                        description: None,
                                        role: None,
                                        preview: None,
                                    }],
                                    free_text: false,
                                    multi: true,
                                },
                            ],
                        },
                        answers: vec![AnswerSpec::Form, AnswerSpec::Cancel],
                        ..interaction.clone()
                    },
                },
            ),
            frame(
                27,
                Event::InteractionResolved {
                    id: InteractionId::from_raw("int_3"),
                    answer: Answer::Form {
                        answers: vec![
                            Answer::Choice {
                                ids: vec!["0".into()],
                                other: None,
                            },
                            Answer::Cancel,
                        ],
                    },
                    by: ResolvedBy::Kernel,
                },
            ),
            // A question ticked and typed on at once: both halves travel (M59).
            frame(
                28,
                Event::InteractionResolved {
                    id: InteractionId::from_raw("int_3"),
                    answer: Answer::Form {
                        answers: vec![Answer::Choice {
                            ids: vec!["0".into(), "1".into()],
                            other: Some("and a place of my own".into()),
                        }],
                    },
                    by: ResolvedBy::Client {
                        name: "tui-1".into(),
                        surface: "tui".into(),
                    },
                },
            ),
            // A shell line the person ran themselves: outside any turn, with
            // the code it came to beside what it wrote (M65).
            frame(
                29,
                Event::ItemCompleted {
                    item: Item {
                        turn: None,
                        ..item(
                            "itm_5",
                            ItemBody::Shell {
                                command: "git status --short".into(),
                                output: " M src/lib.rs\n".into(),
                                exit: Some(0),
                                cwd: "/tmp/p".into(),
                            },
                        )
                    },
                },
            ),
        ];
        insta::assert_json_snapshot!("frames", frames);
        for f in &frames {
            let json = serde_json::to_string(f).unwrap();
            assert_eq!(&serde_json::from_str::<Frame>(&json).unwrap(), f);
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
