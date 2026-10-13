//! What a room is, read off the kernel's own facts. A room is a `Log` session
//! (ADR-0011 §1) under a person's, keyed `rooms/…` and titled `#name`; its
//! members are the latest `members` extension published into its journal, and
//! nothing here keeps a copy of either beside them.
//!
//! Three kinds say the whole of it: `members` is who is in it, `opened` why it
//! is there, and `closed` that it has ended (ADR-0053 §1 and §4). Each is read
//! back here and nowhere else, so no caller can hold a second idea of one.

use bingo_sdk::{Driver, SessionId, SessionState, SessionSummary, View};
use jiff::Timestamp;
use serde_json::{Value, json};

pub use crate::cursor::{Cursor, of_state as cursor};
use crate::ear::{self, Ears, Seat};
pub use crate::ear::{Ear, Listener};
pub const PLUGIN: &str = "bingo.rooms";

/// The one kind this plugin publishes for a room as a whole. A payload is the
/// whole of a room's membership (ADR-0011 §2), so writing it replaces it.
pub const MEMBERS: &str = "members";

/// Why the room was opened, and who opened it (ADR-0053 §1): one purpose per
/// room, read at the head of every reading, and the signing name the roster
/// verbs check a caller against.
pub const OPENED: &str = "opened";

/// That the room has ended (ADR-0053 §4). The frame is the whole of what
/// closing means: the session is not ended and nothing is deleted, because the
/// journal is the record.
pub const CLOSED: &str = "closed";

/// The fields those two payloads carry. They are read by hand out of a
/// persisted journal, so each is written down once.
const PURPOSE: &str = "purpose";
const BY: &str = "by";
const AT: &str = "at";
const WHY: &str = "why";

/// The seats in that payload that are not live, with the patience each asked
/// for (ADR-0029 §2). A payload without it is an all-live roster.
pub const LISTENERS: &str = "listeners";

/// The kind one seat's own retuning is published under, before its name: a
/// register per seat, so no two seats write over each other (ADR-0029 §4).
pub const EAR: &str = "ear:";

/// The first segment of a room's key; a store key is `owner/path`, and this
/// plugin owns `rooms`.
pub const KEY: &str = "rooms/";

/// Why a room is there, as the frame that opened it says (ADR-0053 §1).
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Opened {
    /// The one thing it is for. A person's `/room` and a team file may say
    /// none, and then the readings carry none either.
    pub purpose: Option<String>,
    /// The signing name of whoever opened it — `parent` for a person's door —
    /// which with the holder is who may change its roster (ADR-0053 §5).
    pub by: String,
}

impl Opened {
    pub fn payload(&self) -> Value {
        json!({ PURPOSE: self.purpose, BY: self.by })
    }

    /// What an opening payload says, or nothing for one that is not an
    /// opening: `by` is what the roster rule reads, so a payload without it
    /// answers nobody's question.
    pub fn of_payload(payload: &Value) -> Option<Opened> {
        Some(Opened {
            purpose: payload[PURPOSE].as_str().map(str::to_string),
            by: payload[BY].as_str()?.to_string(),
        })
    }

    pub fn of_state(state: &SessionState) -> Option<Opened> {
        Opened::of_payload(published(state, OPENED)?)
    }
}

/// The end of a room, as the frame that closed it says (ADR-0053 §4).
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Closed {
    pub at: Timestamp,
    pub by: String,
    /// What was said about it, where the caller said anything.
    pub why: Option<String>,
}

impl Closed {
    pub fn payload(&self) -> Value {
        json!({ AT: self.at.to_string(), BY: self.by, WHY: self.why })
    }

    /// What a closing payload says, or nothing for one that says nothing this
    /// build can read. Whether the room is closed is [`closed_of`]'s answer
    /// and never this one's: a payload nothing here understands closed the
    /// room all the same.
    pub fn of_payload(payload: &Value) -> Option<Closed> {
        Some(Closed {
            at: payload[AT].as_str()?.parse().ok()?,
            by: payload[BY].as_str()?.to_string(),
            why: payload[WHY].as_str().map(str::to_string),
        })
    }

    pub fn of_state(state: &SessionState) -> Option<Closed> {
        Closed::of_payload(published(state, CLOSED)?)
    }
}

/// A room, as its own journal says it is.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Room {
    /// `#design`: the name a member is told a post came from.
    pub title: String,
    /// The session the room hangs under. Its members are that session's other
    /// children, so a room reaches exactly as far as the tree it sits in.
    pub parent: SessionId,
    pub members: Vec<String>,
    /// What each of them hears (ADR-0029 §1).
    pub ears: Ears,
    /// The one thing it was opened for, where its opener said one.
    pub purpose: Option<String>,
    /// Whether it has ended (ADR-0053 §4): a closed room is read to its end
    /// and then never, takes no post, owes nothing and is chased for nothing.
    pub closed: bool,
}

impl Room {
    /// The room a summary announces, or `None` for a session that is not one.
    pub fn of(summary: &SessionSummary) -> Option<Room> {
        if summary.driver != Driver::Log {
            return None;
        }
        if !summary
            .key
            .as_deref()
            .is_some_and(|key| key.starts_with(KEY))
        {
            return None;
        }
        Some(Room {
            title: summary.title.clone()?,
            parent: summary.parent.as_ref()?.session.clone(),
            // A summary says who a room is, never who is in it, what it is for
            // or whether it still stands: the frames that follow it do.
            members: Vec::new(),
            ears: Ears::default(),
            purpose: None,
            closed: false,
        })
    }

    /// The seats it has: its names, each wearing the ear it now hears with.
    /// The same roster [`roster_of`] reads off a snapshot, for a caller that
    /// holds the room rather than the snapshot it came from.
    pub fn seats(&self) -> Vec<Seat> {
        self.members
            .iter()
            .map(|name| Seat {
                ear: self.ears.of(name),
                name: name.clone(),
            })
            .collect()
    }

    /// The same room, as its own journal has it. A summary says who a room is
    /// and nothing else about it, so a reader that has only just met one fills
    /// the rest in from the snapshot.
    pub fn seated(&self, state: &SessionState) -> Room {
        Room {
            members: members_of(state),
            ears: ear::ears_of(state),
            purpose: purpose_of(state),
            closed: closed_of(state),
            ..self.clone()
        }
    }
}

/// One of this plugin's kinds in a room's journal, as the snapshot folded it.
fn published<'a>(state: &'a SessionState, kind: &str) -> Option<&'a Value> {
    state.extensions.get(crate::PLUGIN)?.get(kind)
}

/// What a room is for, or nothing for a room whose opener said nothing.
pub fn purpose_of(state: &SessionState) -> Option<String> {
    Opened::of_state(state)?.purpose
}

/// Whether a room has been closed. The frame's presence is the whole of the
/// answer (ADR-0053 §4), so a payload no build understands still closes it.
pub fn closed_of(state: &SessionState) -> bool {
    is_closed(published(state, CLOSED))
}

/// The same question of one payload, for the fold that sees frames rather than
/// snapshots.
pub fn is_closed(payload: Option<&Value>) -> bool {
    payload.is_some_and(|payload| !payload.is_null())
}

/// Who is in a room, as the room's own journal has it. Every reader — the
/// command, the fold, a test — comes through here, so none of them can hold a
/// second idea of a membership.
pub fn members_of(state: &SessionState) -> Vec<String> {
    state
        .extensions
        .get(crate::PLUGIN)
        .and_then(|kinds| kinds.get(MEMBERS))
        .map(members_from)
        .unwrap_or_default()
}

/// The names a `members` payload holds. Anything else in it is not a name.
pub fn members_from(payload: &Value) -> Vec<String> {
    payload[MEMBERS]
        .as_array()
        .map(|names| {
            names
                .iter()
                .filter_map(Value::as_str)
                .map(str::to_string)
                .collect()
        })
        .unwrap_or_default()
}

/// The roster a room now has: its names, each wearing the ear its journal
/// gives it.
pub fn roster_of(state: &SessionState) -> Vec<Seat> {
    let ears = ear::ears_of(state);
    members_of(state)
        .into_iter()
        .map(|name| Seat {
            ear: ears.of(&name),
            name,
        })
        .collect()
}

/// A membership as it is published: the whole of it, under one key. A roster
/// of live seats keeps the names it was written with before there were ears,
/// and wears the tree it is drawn as (ADR-0013 §2) in the same payload.
pub fn payload(seats: &[Seat]) -> Value {
    let names: Vec<&str> = seats.iter().map(|seat| seat.name.as_str()).collect();
    let mut payload = drawn(seats);
    payload[MEMBERS] = json!(names);
    if let Some(listeners) = ear::listeners_of(seats) {
        payload[LISTENERS] = listeners;
    }
    bingo_sdk::slot::with(payload, [])
}

/// The roster in the vocabulary every surface already draws, so `ctrl+t` and
/// a rail card get a tree rather than the raw object. It rides in the same
/// payload as the names, minted here from the same seats: there is one way to
/// write a roster and it cannot write half of one.
///
/// It is the roster *as declared*. A seat that has retuned its own ear since
/// publishes that under its own `ear:` kind (ADR-0029 §4) — folding it back
/// in would mean rewriting the roster on every retune, which is the race that
/// register was split out to avoid — so what delivery hears is
/// [`ear::ears_of`], and this tree is what the room was seated with.
fn drawn(seats: &[Seat]) -> Value {
    let view = View::Tree {
        nodes: ear::nodes(seats),
    };
    serde_json::to_value(view).unwrap_or_default()
}

/// The complete membership payload, including its surface view.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct Members {
    pub members: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub listeners: Option<Vec<Listener>>,
    #[serde(flatten)]
    pub view: serde_json::Map<String, Value>,
}

/// One seat's journaled ear delta.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct Retuned {
    pub patience_s: u64,
}

mod readers;
pub use crate::name::PARENT as HOLDER;
pub use readers::{Hearing, Owes, debts, hearing, unread_count};

#[cfg(test)]
mod m148 {
    use super::*;
    #[test]
    fn m148_members_are_unplaced_and_readable() {
        let value = payload(&[Seat::live("scout")]);
        assert_eq!(value["slots"], serde_json::json!([]));
        assert_eq!(members_from(&value), ["scout"]);
    }
}
