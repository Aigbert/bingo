//! The one reducer. The kernel maintains its snapshot with it; every client
//! folds frames with it. A client's view is always `apply(snapshot, frames
//! since snapshot.seq)`, so no surface needs rules of its own.

use std::collections::BTreeMap;

use jiff::Timestamp;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::Transcript;
use crate::event::*;
use crate::ids::{IntentId, InteractionId, ItemId, Seq, TurnId};
use crate::model::Usage;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct SessionState {
    /// The last frame applied. Frames at or below this are duplicates.
    pub seq: Seq,
    pub summary: SessionSummary,
    #[serde(default)]
    pub config: ConfigView,
    #[serde(flatten)]
    pub transcript: Transcript,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub turn: Option<LiveTurn>,
    #[serde(default)]
    pub queue: Vec<QueueEntry>,
    /// Open interactions, in the order they were opened.
    #[serde(default)]
    pub interactions: Vec<Interaction>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub context: Option<ContextUsage>,
    /// The turn that last ended, whole: what a surface says after a turn is
    /// drawn from this rather than kept by the surface (M84).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_turn: Option<LastTurn>,
    /// A turn ended and nobody has looked since. Cleared by `mark_read`.
    #[serde(default)]
    pub unread: bool,
    #[serde(default)]
    pub closed: bool,
    /// Plugin-owned state, by plugin then kind: the latest `Extension`
    /// payload is the whole of that kind (ADR-0011 §2).
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub extensions: BTreeMap<String, BTreeMap<String, Value>>,
    /// Plugin-owned live state, by plugin then kind: the latest `Signal`
    /// payload, gone on `Null` and after a resume (ADR-0013 §2).
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub invalidated: BTreeMap<String, u64>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    #[schemars(with = "BTreeMap<String, BTreeMap<String, Value>>")]
    pub signals: BTreeMap<String, BTreeMap<String, LiveSignal>>,
}

/// Server-side ownership stays beside the payload and never changes its wire shape.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(transparent)]
pub struct LiveSignal {
    #[serde(skip)]
    generation: Option<ItemId>,
    payload: Value,
}
impl LiveSignal {
    pub fn generation(&self) -> Option<&ItemId> {
        self.generation.as_ref()
    }
    pub fn payload(&self) -> &Value {
        &self.payload
    }
}
impl std::ops::Deref for LiveSignal {
    type Target = Value;
    fn deref(&self) -> &Value {
        self.payload()
    }
}
impl std::ops::DerefMut for LiveSignal {
    fn deref_mut(&mut self) -> &mut Value {
        &mut self.payload
    }
}
impl From<Value> for LiveSignal {
    fn from(payload: Value) -> Self {
        Self {
            generation: None,
            payload,
        }
    }
}
impl PartialEq<Value> for LiveSignal {
    fn eq(&self, value: &Value) -> bool {
        self.payload() == value
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct LiveTurn {
    pub id: TurnId,
    #[schemars(with = "String")]
    pub started_at: Timestamp,
    pub origin: TurnOrigin,
    #[serde(default)]
    pub round: u32,
    #[serde(default)]
    pub usage: Usage,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub retrying: Option<Retry>,
}

/// The turn that last ended: its verdict, the clocks it ran between and what
/// it spent, folded from the turn it closes and the frame that closed it.
/// Nothing here is on the wire that was not already — the clocks are two
/// frames' own, the usage is `TurnCompleted`'s — so a row that says `Worked
/// for 15m 11s` is derived, as every view is (ADR-0002).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct LastTurn {
    pub id: TurnId,
    pub status: TurnStatus,
    #[schemars(with = "String")]
    pub started_at: Timestamp,
    #[schemars(with = "String")]
    pub ended_at: Timestamp,
    #[serde(default)]
    pub usage: Usage,
}

/// A stream attempt failed and the turn is waiting to try again.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct Retry {
    pub attempt: u32,
    pub max: u32,
}

/// What a frame changed, so a renderer can redraw only that.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Applied {
    /// Already applied or older than the snapshot.
    Stale,
    Nothing,
    Session,
    Config,
    Turn,
    Item(ItemId),
    Queue,
    Interaction(InteractionId),
    Intent(IntentId),
    History,
    Notice,
    Extension,
    Signal,
    Lagged,
}

impl SessionState {
    pub fn new(summary: SessionSummary) -> Self {
        Self {
            seq: Seq::ZERO,
            summary,
            config: ConfigView::default(),
            transcript: Transcript::new(),
            turn: None,
            queue: Vec::new(),
            interactions: Vec::new(),
            context: None,
            last_turn: None,
            unread: false,
            closed: false,
            extensions: BTreeMap::new(),
            signals: BTreeMap::new(),
            invalidated: BTreeMap::new(),
        }
    }

    /// Derived, never stored: something needs a person.
    pub fn attention(&self) -> bool {
        !self.interactions.is_empty() || (self.turn.is_none() && self.unread)
    }

    pub fn mark_read(&mut self) {
        self.unread = false;
    }

    /// Derived, never stored: a turn is running. The same fact reaches a
    /// client that holds only a [`SessionSummary`] as `summary.busy`, which
    /// the kernel stamps; the reducer writes it nowhere.
    /// The verdict of the turn that last ended, for the readers that want
    /// only that.
    pub fn last_status(&self) -> Option<&TurnStatus> {
        self.last_turn.as_ref().map(|turn| &turn.status)
    }

    pub fn busy(&self) -> bool {
        self.turn.is_some()
    }

    pub fn item(&self, id: &ItemId) -> Option<&Item> {
        self.transcript.get(id)
    }

    pub fn apply(&mut self, frame: &Frame) -> Applied {
        // A lag marker is transport, not history: it must not move `seq`, so
        // the client's next `events_since(seq)` replays what it missed.
        if matches!(frame.event, Event::Lagged { .. }) {
            return Applied::Lagged;
        }
        if frame.seq <= self.seq && self.seq != Seq::ZERO {
            return Applied::Stale;
        }
        self.seq = frame.seq;
        if frame.event.completes_a_message() {
            self.count_a_message(&frame.event);
        }
        let applied = self.transcript.apply(&frame.event);
        match &frame.event {
            Event::SessionUpdated { summary } => self.session_updated(summary),
            Event::SessionClosed { .. } => self.session_closed(),
            Event::TurnStarted { turn, origin, .. } => self.turn_started(turn, *origin, frame.ts),
            Event::TurnRetrying { attempt, max, .. } => self.turn_retrying(*attempt, *max),
            Event::TurnUsage { usage, context, .. } => self.turn_usage(*usage, *context),
            Event::UsageCharged { turn, usage, .. } => self.usage_charged(turn.as_ref(), *usage),
            Event::TurnCompleted {
                turn,
                status,
                usage,
            } => self.turn_completed(turn, status, *usage, frame.ts),
            Event::ItemStarted { .. }
            | Event::ItemUpdated { .. }
            | Event::ItemCompleted { .. }
            | Event::ItemDelta { .. }
            | Event::Compacted { .. } => applied,
            Event::QueueChanged { entries, .. } => self.queue_changed(entries),
            Event::InteractionOpened { interaction } => self.interaction_opened(interaction),
            Event::InteractionResolved { id, .. } => self.interaction_closed(id),
            Event::InteractionCancelled { id, .. } => self.interaction_closed(id),
            Event::IntentAck { intent, .. } => Applied::Intent(intent.clone()),
            Event::Rewound { .. } => self.rewound(applied),
            Event::ConfigChanged { config } => self.config_changed(config),
            Event::CatalogChanged { .. } | Event::Unknown { .. } => Applied::Nothing,
            Event::Notice { .. } => Applied::Notice,
            Event::Extension {
                plugin,
                kind,
                payload,
            } => self.extended(plugin, kind, payload),
            Event::Signal {
                generation,
                plugin,
                kind,
                payload,
            } => self.signalled(plugin, kind, payload, generation.as_ref()),
            Event::Invalidated { requests } => self.invalidate(requests),
            Event::Lagged { .. } => Applied::Lagged,
        }
    }

    fn invalidate(&mut self, requests: &[String]) -> Applied {
        for request in requests {
            let generation = self.invalidated.entry(request.clone()).or_default();
            *generation = generation.saturating_add(1);
        }
        Applied::Signal
    }

    fn extended(&mut self, plugin: &str, kind: &str, payload: &Value) -> Applied {
        if payload.is_null() {
            if let Some(kinds) = self.extensions.get_mut(plugin) {
                kinds.remove(kind);
                if kinds.is_empty() {
                    self.extensions.remove(plugin);
                }
            }
        } else {
            self.extensions
                .entry(plugin.to_string())
                .or_default()
                .insert(kind.to_string(), payload.clone());
        }
        Applied::Extension
    }

    fn signalled(
        &mut self,
        plugin: &str,
        kind: &str,
        payload: &Value,
        generation: Option<&ItemId>,
    ) -> Applied {
        if payload.is_null() {
            if let Some(kinds) = self.signals.get_mut(plugin) {
                if generation.is_some_and(|id| {
                    kinds.get(kind).and_then(|signal| signal.generation()) != Some(id)
                }) {
                    return Applied::Nothing;
                }
                kinds.remove(kind);
                if kinds.is_empty() {
                    self.signals.remove(plugin);
                }
            }
        } else {
            self.signals.entry(plugin.to_string()).or_default().insert(
                kind.to_string(),
                LiveSignal {
                    generation: generation.cloned(),
                    payload: payload.clone(),
                },
            );
        }
        Applied::Signal
    }

    /// A journal that was never counted starts counting here; what came before
    /// this fold is lost either way, and a rebuild from the whole journal is
    /// what puts an old session's true number back.
    fn count_a_message(&mut self, event: &Event) {
        let history = &mut self.summary.history;
        history.messages = Some(history.messages.unwrap_or(0) + 1);
        if history.first_ask.is_none()
            && let Event::ItemCompleted { item } = event
        {
            history.first_ask = crate::first_ask(item);
        }
    }

    /// Only a running session says what it is, so a summary after a
    /// `SessionClosed` is a new segment of the journal: the close ended the
    /// process that wrote it, not the session, and it is open again by
    /// saying so.
    fn session_updated(&mut self, summary: &SessionSummary) -> Applied {
        let first_ask = self
            .summary
            .history
            .first_ask
            .clone()
            .or_else(|| summary.history.first_ask.clone());
        self.summary = summary.clone();
        self.summary.history.first_ask = first_ask;
        self.closed = false;
        Applied::Session
    }

    fn session_closed(&mut self) -> Applied {
        self.closed = true;
        self.turn = None;
        self.interactions.clear();
        Applied::Session
    }

    fn turn_started(
        &mut self,
        turn: &TurnId,
        origin: TurnOrigin,
        started_at: Timestamp,
    ) -> Applied {
        self.turn = Some(LiveTurn {
            id: turn.clone(),
            started_at,
            origin,
            round: 0,
            usage: Usage::default(),
            retrying: None,
        });
        Applied::Turn
    }

    fn turn_retrying(&mut self, attempt: u32, max: u32) -> Applied {
        if let Some(t) = self.turn.as_mut() {
            t.retrying = Some(Retry { attempt, max });
        }
        Applied::Turn
    }

    fn usage_charged(&mut self, turn: Option<&TurnId>, usage: Usage) -> Applied {
        if let Some(live) = self.turn.as_mut().filter(|live| Some(&live.id) == turn) {
            live.usage.add(usage);
        }
        self.summary.usage.add(usage);
        Applied::Turn
    }

    fn turn_usage(&mut self, usage: Usage, context: ContextUsage) -> Applied {
        if let Some(t) = self.turn.as_mut() {
            t.usage.add(usage);
            t.round += 1;
            t.retrying = None;
        }
        self.summary.usage.add(usage);
        self.context = Some(context);
        Applied::Turn
    }

    /// The live turn is put away as the last one. A completion for a turn
    /// this state never saw start — a journal read from the middle — began,
    /// as far as anyone here knows, when it ended.
    fn turn_completed(
        &mut self,
        turn: &TurnId,
        status: &TurnStatus,
        usage: Usage,
        ended_at: Timestamp,
    ) -> Applied {
        let started_at = self
            .turn
            .take()
            .filter(|live| live.id == *turn)
            .map_or(ended_at, |live| live.started_at);
        self.last_turn = Some(LastTurn {
            id: turn.clone(),
            status: status.clone(),
            started_at,
            ended_at,
            usage,
        });
        self.unread = true;
        Applied::Turn
    }

    /// A rewind cuts whole turns, so the last turn was taken back exactly
    /// when none of its items is left: its `Worked for` line goes with it.
    fn rewound(&mut self, applied: Applied) -> Applied {
        let kept = |turn: &LastTurn| {
            self.transcript
                .items()
                .iter()
                .any(|item| item.turn.as_ref() == Some(&turn.id))
        };
        if self.last_turn.as_ref().is_some_and(|turn| !kept(turn)) {
            self.last_turn = None;
        }
        applied
    }

    fn queue_changed(&mut self, entries: &[QueueEntry]) -> Applied {
        self.queue = entries.to_vec();
        Applied::Queue
    }

    fn interaction_opened(&mut self, interaction: &Interaction) -> Applied {
        self.interactions.retain(|i| i.id != interaction.id);
        self.interactions.push(interaction.clone());
        Applied::Interaction(interaction.id.clone())
    }

    fn interaction_closed(&mut self, id: &InteractionId) -> Applied {
        self.interactions.retain(|i| &i.id != id);
        Applied::Interaction(id.clone())
    }

    fn config_changed(&mut self, config: &ConfigView) -> Applied {
        self.config = config.clone();
        Applied::Config
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ids::SessionId;
    use crate::model::ContentPart;

    fn ts() -> Timestamp {
        Timestamp::from_second(1_700_000_000).unwrap()
    }

    fn summary() -> SessionSummary {
        SessionSummary {
            effort: None,
            model_scope: None,
            history: crate::SessionHistory {
                first_ask: None,
                forked_from: None,
                messages: None,
            },
            id: SessionId::from_raw("ses_1"),
            key: None,
            title: None,
            cwd: "/tmp".into(),
            parent: None,
            driver: Driver::Model,
            model: None,
            system_extra: None,
            tools: None,
            provider: None,
            created_at: ts(),
            updated_at: ts(),
            usage: Usage::default(),
            busy: false,
            asking: false,
        }
    }

    #[test]
    fn m148_null_extension_journal_removes_only_its_kind() {
        let events = [
            Event::Extension {
                plugin: "fixture".into(),
                kind: "board".into(),
                payload: serde_json::json!({"n": 1}),
            },
            Event::Extension {
                plugin: "fixture".into(),
                kind: "kept".into(),
                payload: serde_json::json!({"n": 2}),
            },
            Event::Extension {
                plugin: "fixture".into(),
                kind: "board".into(),
                payload: Value::Null,
            },
        ];
        let expected = serde_json::to_string_pretty(&events).unwrap() + "\n";
        let path =
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("fixtures/extensions-null.json");
        if std::env::var_os("BINGO_UPDATE_SCHEMA").is_some() {
            std::fs::write(&path, &expected).unwrap();
        }
        let fixture = std::fs::read_to_string(path).unwrap();
        assert_eq!(fixture, expected);
        let mut state = SessionState::new(summary());
        for (at, event) in serde_json::from_str::<Vec<Event>>(&fixture)
            .unwrap()
            .into_iter()
            .enumerate()
        {
            state.apply(&frame(at as u64 + 1, event));
        }
        assert!(!state.extensions["fixture"].contains_key("board"));
        assert_eq!(
            state.extensions["fixture"]["kept"],
            serde_json::json!({"n": 2})
        );
        state.apply(&frame(
            4,
            Event::Extension {
                plugin: "fixture".into(),
                kind: "kept".into(),
                payload: Value::Null,
            },
        ));
        assert!(state.extensions.is_empty());
    }

    #[test]
    fn side_usage_adds_without_advancing_the_round_or_context() {
        let mut state = SessionState::new(summary());
        state.apply(&frame(
            1,
            Event::TurnStarted {
                turn: TurnId::from_raw("trn_1"),
                inputs: vec![],
                origin: TurnOrigin::Submit,
            },
        ));
        let charge: Event =
            serde_json::from_str(include_str!("../fixtures/record/event/usageCharged.json"))
                .unwrap();
        state.apply(&frame(2, charge));
        assert_eq!(state.summary.usage.input_tokens, 5);
        let turn = state.turn.as_ref().unwrap();
        assert_eq!(turn.usage.input_tokens, 5);
        assert_eq!(turn.round, 0);
        assert!(state.context.is_none());
        let charge = serde_json::from_value(serde_json::json!({
            "type":"usageCharged", "plugin":"stub", "usage":{"inputTokens":2,"outputTokens":1}
        }))
        .unwrap();
        state.apply(&frame(3, charge));
        assert_eq!(state.summary.usage.input_tokens, 7);
        assert_eq!(state.turn.as_ref().unwrap().usage.input_tokens, 5);
    }

    #[test]
    fn transcript_snapshot_keeps_the_wire_bytes() {
        let mut state = SessionState::new(summary());
        state.apply(&frame(
            1,
            Event::ItemCompleted {
                item: assistant("i1", "hello", ItemStatus::Completed),
            },
        ));
        state.apply(&frame(
            2,
            Event::Compacted {
                generation: 3,
                boundary: ItemId::from_raw("i1"),
                summary: ItemId::from_raw("missing"),
                kept: vec![],
            },
        ));
        let wire = serde_json::to_string(&state).unwrap();
        assert_eq!(
            wire,
            r#"{"seq":2,"summary":{"id":"ses_1","cwd":"/tmp","driver":"model","createdAt":"2023-11-14T22:13:20Z","updatedAt":"2023-11-14T22:13:20Z","usage":{"inputTokens":0,"outputTokens":0,"cacheReadTokens":0,"cacheWriteTokens":0,"reasoningTokens":0},"busy":false,"asking":false,"messages":1},"config":{"kernel":null,"plugins":{}},"historyGeneration":3,"items":[{"id":"i1","turn":"trn_1","round":0,"status":"completed","startedAt":"2023-11-14T22:13:20Z","body":{"kind":"assistant","text":"hello"}}],"queue":[],"interactions":[],"unread":false,"closed":false}"#
        );
        assert_eq!(
            serde_json::to_string(&serde_json::from_str::<SessionState>(&wire).unwrap()).unwrap(),
            wire
        );
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

    fn assistant(id: &str, text: &str, status: ItemStatus) -> Item {
        Item {
            id: ItemId::from_raw(id),
            turn: Some(TurnId::from_raw("trn_1")),
            round: 0,
            status,
            started_at: ts(),
            completed_at: None,
            intent: None,
            body: ItemBody::Assistant { text: text.into() },
            meta: Default::default(),
        }
    }

    fn user(id: &str, text: &str) -> Item {
        Item {
            body: ItemBody::User {
                parts: vec![ContentPart::text(text)],
                origin: Origin::surface("tui"),
            },
            ..assistant(id, "", ItemStatus::Completed)
        }
    }

    fn tool_call(id: &str) -> Item {
        Item {
            body: ItemBody::ToolCall {
                call_id: "call_1".into(),
                name: "Read".into(),
                input: Value::Null,
                output: None,
                progress: None,
                duration_ms: None,
                quiet: false,
            },
            ..assistant(id, "", ItemStatus::Completed)
        }
    }

    /// One item, three frames on its way to done: the count moves once.
    #[test]
    fn a_message_is_counted_where_it_is_completed_and_only_there() {
        let mut st = SessionState::new(summary());
        assert_eq!(
            st.summary.history.messages, None,
            "a summary that never counted"
        );
        st.apply(&frame(
            1,
            Event::ItemCompleted {
                item: user("itm_1", "fix the parser"),
            },
        ));
        let running = assistant("itm_2", "", ItemStatus::Running);
        st.apply(&frame(
            2,
            Event::ItemStarted {
                item: running.clone(),
            },
        ));
        st.apply(&frame(3, Event::ItemUpdated { item: running }));
        st.apply(&frame(
            4,
            Event::ItemCompleted {
                item: assistant("itm_2", "done", ItemStatus::Completed),
            },
        ));
        st.apply(&frame(
            5,
            Event::ItemCompleted {
                item: tool_call("itm_3"),
            },
        ));
        assert_eq!(
            st.summary.history.messages,
            Some(2),
            "an ask and an answer; the tool call between them is not a message"
        );
    }

    /// The head of a new segment carries the count the last one reached, so a
    /// resumed session does not start again from nought.
    #[test]
    fn a_session_frame_hands_the_count_on() {
        let mut st = SessionState::new(summary());
        st.apply(&frame(
            1,
            Event::ItemCompleted {
                item: user("itm_1", "one"),
            },
        ));
        let head = SessionSummary {
            effort: None,
            model_scope: None,
            history: crate::SessionHistory {
                messages: st.summary.history.messages,
                ..summary().history
            },
            ..summary()
        };
        st.apply(&frame(2, Event::SessionUpdated { summary: head }));
        assert_eq!(st.summary.history.messages, Some(1));
        st.apply(&frame(
            3,
            Event::ItemCompleted {
                item: user("itm_2", "two"),
            },
        ));
        assert_eq!(st.summary.history.messages, Some(2));
    }

    /// The close at the end of a segment is not the end of the session: the
    /// summary at the head of the next one opens it again, and what it said
    /// before the close is still what it said.
    #[test]
    fn a_session_frame_after_a_close_opens_the_session_again() {
        let mut st = SessionState::new(summary());
        st.apply(&frame(
            1,
            Event::ItemCompleted {
                item: user("itm_1", "one"),
            },
        ));
        st.apply(&frame(
            2,
            Event::SessionClosed {
                reason: crate::event::CloseReason::Shutdown,
            },
        ));
        assert!(st.closed);
        st.apply(&frame(3, Event::SessionUpdated { summary: summary() }));
        assert!(!st.closed, "a summary comes only from a running session");
        assert_eq!(st.transcript.items().len(), 1, "the close rewrote nothing");
    }

    #[test]
    fn only_prose_and_answers_are_messages() {
        assert!(user("itm_1", "hi").body.is_message());
        assert!(
            assistant("itm_2", "hello", ItemStatus::Completed)
                .body
                .is_message()
        );
        assert!(!tool_call("itm_3").body.is_message());
        assert!(
            !ItemBody::Notice {
                level: Level::Info,
                code: "X".into(),
                text: "x".into(),
            }
            .is_message()
        );
    }

    #[test]
    fn deltas_accumulate_until_the_authoritative_completion() {
        let mut st = SessionState::new(summary());
        st.apply(&frame(
            1,
            Event::TurnStarted {
                turn: TurnId::from_raw("trn_1"),
                inputs: vec![],
                origin: TurnOrigin::Submit,
            },
        ));
        assert!(st.busy());
        st.apply(&frame(
            2,
            Event::ItemStarted {
                item: assistant("itm_1", "", ItemStatus::Running),
            },
        ));
        st.apply(&frame(
            3,
            Event::ItemDelta {
                item: ItemId::from_raw("itm_1"),
                n: 0,
                kind: DeltaKind::Text,
                data: "Hel".into(),
            },
        ));
        st.apply(&frame(
            4,
            Event::ItemDelta {
                item: ItemId::from_raw("itm_1"),
                n: 1,
                kind: DeltaKind::Text,
                data: "lo".into(),
            },
        ));
        assert_eq!(
            st.item(&ItemId::from_raw("itm_1")).unwrap().body,
            ItemBody::Assistant {
                text: "Hello".into()
            }
        );
        st.apply(&frame(
            5,
            Event::ItemCompleted {
                item: assistant("itm_1", "Hello!", ItemStatus::Completed),
            },
        ));
        let late = st.apply(&frame(
            6,
            Event::ItemDelta {
                item: ItemId::from_raw("itm_1"),
                n: 2,
                kind: DeltaKind::Text,
                data: "??".into(),
            },
        ));
        assert_eq!(late, Applied::Nothing);
        assert_eq!(
            st.item(&ItemId::from_raw("itm_1")).unwrap().body,
            ItemBody::Assistant {
                text: "Hello!".into()
            }
        );
        st.apply(&frame(
            7,
            Event::TurnCompleted {
                turn: TurnId::from_raw("trn_1"),
                status: TurnStatus::Completed,
                usage: Usage::default(),
            },
        ));
        assert!(!st.busy());
        assert!(
            st.attention(),
            "a finished turn nobody looked at needs attention"
        );
        st.mark_read();
        assert!(!st.attention());
    }

    #[test]
    fn stale_and_duplicate_frames_are_ignored() {
        let mut st = SessionState::new(summary());
        st.apply(&frame(
            5,
            Event::ConfigChanged {
                config: ConfigView::default(),
            },
        ));
        assert_eq!(
            st.apply(&frame(5, Event::CatalogChanged { kind: "x".into() })),
            Applied::Stale
        );
        assert_eq!(
            st.apply(&frame(3, Event::CatalogChanged { kind: "x".into() })),
            Applied::Stale
        );
        assert_eq!(st.seq, Seq(5));
    }

    #[test]
    fn a_lag_marker_leaves_seq_where_the_last_applied_frame_put_it() {
        let mut st = SessionState::new(summary());
        st.apply(&frame(
            3,
            Event::ConfigChanged {
                config: ConfigView::default(),
            },
        ));
        assert_eq!(
            st.apply(&frame(
                9,
                Event::Lagged {
                    from: Seq(4),
                    to: Seq(9)
                }
            )),
            Applied::Lagged
        );
        assert_eq!(
            st.seq,
            Seq(3),
            "resync must start from the last applied frame"
        );
        assert_ne!(
            st.apply(&frame(4, Event::CatalogChanged { kind: "x".into() })),
            Applied::Stale
        );
    }

    #[test]
    fn a_changed_answer_waits_for_item_updated_to_change_the_call() {
        let mut state = SessionState::new(summary());
        state.apply(&frame(
            1,
            Event::ItemStarted {
                item: tool_call("call"),
            },
        ));
        let interaction: Interaction = serde_json::from_value(serde_json::json!({
            "id":"ask", "session":"ses_1", "item":"call", "openedAt":ts().to_string(),
            "kind":{"kind":"permission","tool":"Read","summary":"read"},
            "answers":["allowChanged"]
        }))
        .unwrap();
        state.apply(&frame(2, Event::InteractionOpened { interaction }));
        let original = state
            .transcript
            .get(&ItemId::from_raw("call"))
            .unwrap()
            .clone();
        let replacement = serde_json::json!({"path":"new"});
        state.apply(&frame(
            3,
            Event::InteractionResolved {
                id: InteractionId::from_raw("ask"),
                answer: Answer::AllowChanged {
                    input: replacement.clone(),
                },
                by: ResolvedBy::Kernel,
            },
        ));
        assert!(state.interactions.is_empty());
        assert!(
            matches!(&state.transcript.get(&ItemId::from_raw("call")).unwrap().body, ItemBody::ToolCall { input, .. } if input != &replacement)
        );
        assert_eq!(state.transcript.get(&original.id), Some(&original));
        let mut changed = original;
        if let ItemBody::ToolCall { input, .. } = &mut changed.body {
            *input = replacement;
        }
        state.apply(&frame(
            4,
            Event::ItemUpdated {
                item: changed.clone(),
            },
        ));
        assert_eq!(state.transcript.get(&changed.id), Some(&changed));
    }

    #[test]
    fn interactions_open_and_close_by_id() {
        let mut st = SessionState::new(summary());
        let interaction = Interaction {
            id: InteractionId::from_raw("int_1"),
            session: SessionId::from_raw("ses_1"),
            turn: None,
            item: None,
            opened_at: ts(),
            guard_until: None,
            expires_at: None,
            kind: InteractionKind::Confirm {
                title: "t".into(),
                detail: "d".into(),
            },
            answers: vec![AnswerSpec::Confirm, AnswerSpec::Cancel],
        };
        st.apply(&frame(1, Event::InteractionOpened { interaction }));
        assert!(st.attention());
        st.apply(&frame(
            2,
            Event::InteractionResolved {
                id: InteractionId::from_raw("int_1"),
                answer: Answer::Confirm,
                by: ResolvedBy::Kernel,
            },
        ));
        assert!(st.interactions.is_empty());
        assert!(!st.attention());
    }

    #[test]
    fn rewind_drops_items_and_bumps_the_generation() {
        let mut st = SessionState::new(summary());
        st.apply(&frame(
            1,
            Event::ItemCompleted {
                item: Item {
                    body: ItemBody::User {
                        parts: vec![ContentPart::text("a")],
                        origin: Origin::surface("tui"),
                    },
                    ..assistant("itm_1", "", ItemStatus::Completed)
                },
            },
        ));
        st.apply(&frame(
            2,
            Event::ItemCompleted {
                item: assistant("itm_2", "b", ItemStatus::Completed),
            },
        ));
        st.apply(&frame(
            3,
            Event::Rewound {
                generation: 1,
                to_turn: TurnId::from_raw("trn_1"),
                dropped: vec![ItemId::from_raw("itm_2")],
                files_restored: vec![],
            },
        ));
        assert_eq!(st.transcript.items().len(), 1);
        assert_eq!(st.transcript.generation(), 1);
    }

    #[test]
    fn rewinding_the_last_turn_forgets_how_long_it_worked() {
        let mut st = SessionState::new(summary());
        for (seq, id, turn) in [(1, "itm_1", "trn_1"), (2, "itm_2", "trn_2")] {
            st.apply(&frame(
                seq,
                Event::ItemCompleted {
                    item: Item {
                        turn: Some(TurnId::from_raw(turn)),
                        ..assistant(id, "said", ItemStatus::Completed)
                    },
                },
            ));
        }
        let completed = |turn: &str| Event::TurnCompleted {
            turn: TurnId::from_raw(turn),
            status: TurnStatus::Completed,
            usage: Usage::default(),
        };
        st.apply(&frame(3, completed("trn_2")));
        let rewound = |generation, to: &str, dropped: &str| Event::Rewound {
            generation,
            to_turn: TurnId::from_raw(to),
            dropped: vec![ItemId::from_raw(dropped)],
            files_restored: vec![],
        };
        st.apply(&frame(4, rewound(1, "trn_2", "itm_2")));
        assert_eq!(st.last_turn, None, "the dropped range held the last turn");
        st.apply(&frame(5, completed("trn_1")));
        st.apply(&frame(6, rewound(2, "trn_9", "itm_none")));
        assert!(
            st.last_turn.is_some(),
            "a cut that keeps the turn keeps its line"
        );
    }

    #[test]
    fn signal_wire_keeps_the_payload_and_hides_internal_ownership() {
        let mut state = SessionState::new(summary());
        let signal = Event::Signal {
            plugin: "fixture".into(),
            kind: "pane".into(),
            generation: Some(ItemId::mint()),
            payload: serde_json::json!({"slots":[], "generation":"plugin data", "payload":true}),
        };
        state.apply(&frame(1, signal.clone()));
        let wire = serde_json::to_value(&state).unwrap();
        assert_eq!(
            wire["signals"]["fixture"]["pane"],
            serde_json::json!({"slots":[], "generation":"plugin data", "payload":true})
        );
        assert!(
            serde_json::to_value(signal)
                .unwrap()
                .get("generation")
                .is_none()
        );
        let decoded: SessionState = serde_json::from_value(wire).unwrap();
        assert_eq!(
            decoded.signals["fixture"]["pane"].payload(),
            state.signals["fixture"]["pane"].payload()
        );
        assert!(decoded.signals["fixture"]["pane"].generation().is_none());
    }

    #[test]
    fn internal_generation_ownership_survives_clones_and_rejects_old_cleanup() {
        let mut st = SessionState::new(summary());
        let old = ItemId::mint();
        let new = ItemId::mint();
        for (seq, generation, payload) in [
            (1, old.clone(), serde_json::json!({"slots":[]})),
            (2, new.clone(), serde_json::json!({"slots":[]})),
            (3, old.clone(), Value::Null),
        ] {
            st.apply(&frame(
                seq,
                Event::Signal {
                    plugin: "fixture".into(),
                    kind: "pane".into(),
                    generation: Some(generation),
                    payload,
                },
            ));
        }
        let mut restored = st.clone();
        assert_eq!(restored.signals["fixture"]["pane"].generation(), Some(&new));
        restored.apply(&frame(
            4,
            Event::Signal {
                plugin: "fixture".into(),
                kind: "pane".into(),
                generation: Some(new),
                payload: Value::Null,
            },
        ));
        assert!(restored.signals.is_empty());
    }

    fn signal(seq: u64, kind: &str, payload: Value) -> Frame {
        frame(
            seq,
            Event::Signal {
                generation: None,
                plugin: "bingo.demo.ui".into(),
                kind: kind.into(),
                payload,
            },
        )
    }

    #[test]
    fn a_signal_keeps_the_latest_per_kind_and_null_removes_it() {
        let mut st = SessionState::new(summary());
        assert_eq!(
            st.apply(&signal(1, "progress", serde_json::json!({"value": 1}))),
            Applied::Signal
        );
        st.apply(&signal(2, "progress", serde_json::json!({"value": 2})));
        st.apply(&signal(3, "board", serde_json::json!({"rows": 0})));
        let demo = &st.signals["bingo.demo.ui"];
        assert_eq!(demo["progress"], serde_json::json!({"value": 2}));
        assert_eq!(demo.len(), 2);

        st.apply(&signal(4, "progress", Value::Null));
        assert_eq!(st.signals["bingo.demo.ui"].len(), 1);
        st.apply(&signal(5, "board", Value::Null));
        assert!(st.signals.is_empty(), "an empty plugin entry goes too");
        assert!(
            st.extensions.is_empty(),
            "a signal never touches the journal's fold"
        );
    }
}
