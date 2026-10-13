//! The ear a seat wears (ADR-0029 §1, ADR-0034 §6). One number says it: 0 is a
//! live ear — every post wakes the seat — and thirty seconds or more is a
//! patient one, whose posts wait to be read at the seat's next turn and wake it
//! once they have waited that long. The band between is refused in words rather
//! than rounded: it names a live seat.
//!
//! The default is the patient ear. A bare name on a roster asks for one, and a
//! live seat is asked for by the number that says so — a room wakes the seats
//! a post names, and the ones that said they wanted every post.
//!
//! A room's ears are two layers, and only two: what the roster declared when it
//! was seated, and what a seat has retuned for itself since (`Listen`). The
//! layers are folded here and nowhere else, so no caller can hold a second idea
//! of what a seat hears.

use std::collections::BTreeMap;
use std::time::Duration;

use bingo_sdk::{ErrorCode, KernelError, SessionState, Tone, TreeNode};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

pub const PATIENCE: Duration = Duration::from_secs(300);
use crate::room::{EAR, LISTENERS, MEMBERS};
use crate::{PLUGIN, name};

/// The shortest patience a seat may ask for. Under it the storm bound a
/// patient ear buys is not worth the wait, so the doors refuse the number
/// instead of quietly rounding it (ADR-0029 §1).
pub const FLOOR: Duration = Duration::from_secs(30);

/// The key a patience is asked for and stored under, in every door.
pub const PATIENCE_S: &str = "patience_s";

/// How a seat hears its room.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Ear {
    /// Every post wakes it, as it lands.
    Live,
    /// Posts wait to be read at the seat's next turn, and the deadline wakes it
    /// once they have waited this long.
    Patient(Duration),
}

impl Default for Ear {
    /// The ear a bare name asks for (ADR-0034 §6): patient, at the chaser's own
    /// patience and not a second constant.
    fn default() -> Ear {
        Ear::Patient(PATIENCE)
    }
}

impl Ear {
    /// The ear a door asks for. A number under the floor is a live seat
    /// described the long way round, and is refused as one.
    pub fn asked(seconds: u64) -> Result<Ear, KernelError> {
        match seconds {
            0 => Ok(Ear::Live),
            _ if seconds < FLOOR.as_secs() => Err(dead_band(seconds)),
            _ => Ok(Ear::Patient(Duration::from_secs(seconds))),
        }
    }

    /// The ear a seat on a roster asks for: a name alone takes the default.
    pub fn declared(seconds: Option<u64>) -> Result<Ear, KernelError> {
        match seconds {
            None => Ok(Ear::default()),
            Some(seconds) => Ear::asked(seconds),
        }
    }

    /// The ear a journal has, which is never refused: what was written was
    /// already accepted at a door, and a number below the floor read back from
    /// an edited journal still asked to be patient, so it is held at the floor
    /// rather than dropped.
    pub fn stored(seconds: u64) -> Ear {
        match seconds {
            0 => Ear::Live,
            _ => Ear::Patient(Duration::from_secs(seconds.max(FLOOR.as_secs()))),
        }
    }

    pub fn is_live(self) -> bool {
        self == Ear::Live
    }

    /// The seconds a payload stores: a live ear is zero patience.
    pub fn seconds(self) -> u64 {
        match self {
            Ear::Live => 0,
            Ear::Patient(patience) => patience.as_secs(),
        }
    }

    /// What the seat is told it now wears.
    pub fn said(self) -> String {
        match self {
            Ear::Live => "every post wakes you as it lands".to_string(),
            Ear::Patient(patience) => format!(
                "you read the room at your next turn, and are woken once it has \
                 stood unread for {}s",
                patience.as_secs()
            ),
        }
    }
}

/// A number nobody should have asked for, and what to ask for instead.
fn dead_band(seconds: u64) -> KernelError {
    KernelError::new(
        ErrorCode::InvalidInput,
        format!(
            "{seconds}s is under thirty seconds of patience: take the live seat you are \
             describing — 0 for an ear every post wakes, or {} and up for a patient one",
            FLOOR.as_secs()
        ),
    )
}

/// A seat on a roster: a name, and the ear it wears.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Seat {
    pub name: String,
    pub ear: Ear,
}

impl Seat {
    /// One word of a roster line: `scout` is a patient seat at the default,
    /// `scout:120` at the patience it asks for, and `scout:0` a live one every
    /// post wakes (ADR-0034 §6).
    pub fn read(word: &str) -> Result<Seat, KernelError> {
        let (name, asked) = match word.split_once(':') {
            Some((name, seconds)) => (name, Some(patience(seconds)?)),
            None => (word, None),
        };
        Ok(Seat {
            name: name.to_string(),
            ear: Ear::declared(asked)?,
        })
    }

    /// A seat every post wakes, which a roster asks for by name.
    pub fn live(name: &str) -> Seat {
        Seat {
            name: name.to_string(),
            ear: Ear::Live,
        }
    }

    /// A seat wearing the ear a bare name asks for.
    pub fn named(name: &str) -> Seat {
        Seat {
            name: name.to_string(),
            ..Seat::default()
        }
    }

    /// How the seat reads back, in a receipt and in `/room`: the word that would
    /// ask for it, which for the default ear is the bare name.
    pub fn said(&self) -> String {
        match self.ear == Ear::default() {
            true => self.name.clone(),
            false => format!("{}:{}", self.name, self.ear.seconds()),
        }
    }
}

/// The seconds after a `name:` — a word, not a number, is nobody's patience.
fn patience(seconds: &str) -> Result<u64, KernelError> {
    seconds.trim().parse().map_err(|_| {
        KernelError::new(
            ErrorCode::InvalidInput,
            format!("{seconds:?} is not a patience: `name` takes the default, `name:120` waits that many seconds, `name:0` wakes for every post"),
        )
    })
}

/// What a room nobody is in reads as, wherever a roster is read back.
pub const NOBODY: &str = "nobody yet";

/// What a seat every post wakes is badged with.
const LIVE: &str = "live";

/// The roster as a person reads it (ADR-0013): a node per seat, in the order
/// the roster seated them, badged with the ear it wears where that is not the
/// room's own default. The tree is drawn here and once, for the block a door
/// answers with and for the roster the room publishes alike.
pub fn nodes(seats: &[Seat]) -> Vec<TreeNode> {
    if seats.is_empty() {
        return vec![leaf(NOBODY, None)];
    }
    seats
        .iter()
        .map(|seat| leaf(&seat.name, badge(seat.ear)))
        .collect()
}

/// One seat as a node. A roster wants nobody, so no seat wears a tone that
/// asks for anyone (ADR-0013 §1).
fn leaf(label: &str, badge: Option<String>) -> TreeNode {
    TreeNode {
        label: label.to_string(),
        badge,
        tone: Tone::Neutral,
        children: Vec::new(),
    }
}

/// What a seat's badge says, and nothing at all for the ear a bare name asks
/// for: a roster is read for what is unusual about it.
fn badge(ear: Ear) -> Option<String> {
    if ear == Ear::default() {
        return None;
    }
    match ear {
        Ear::Live => Some(LIVE.to_string()),
        Ear::Patient(patience) => Some(format!("{}s", patience.as_secs())),
    }
}

/// A listener as the structured doors declare one: a name, or a name and the
/// patience it asks for — zero for a seat every post wakes. `OpenRoom` and
/// `.bingo/team.json` say it the same way.
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(untagged)]
pub enum Listener {
    /// A name alone: the default ear, which is a patient one.
    Name(String),
    Asked {
        name: String,
        patience_s: Option<u64>,
    },
}

impl Listener {
    /// The seat it declares, or the refusal its patience earns.
    pub fn seat(&self) -> Result<Seat, KernelError> {
        let (name, asked) = match self {
            Listener::Name(name) => (name, None),
            Listener::Asked { name, patience_s } => (name, *patience_s),
        };
        Ok(Seat {
            name: name.clone(),
            ear: Ear::declared(asked)?,
        })
    }
}

/// The roster a door declares: its members at the default ear unless its
/// listeners say otherwise. A name in both is one seat, and the ear is the
/// listener's.
pub fn seats(members: &[String], listeners: &[Listener]) -> Result<Vec<Seat>, KernelError> {
    let mut seats: Vec<Seat> = members.iter().map(|m| Seat::named(m)).collect();
    for listener in listeners {
        let seat = listener.seat()?;
        match seats
            .iter_mut()
            .find(|held| name::same(&held.name, &seat.name))
        {
            Some(held) => held.ear = seat.ear,
            None => seats.push(seat),
        }
    }
    Ok(seats)
}

/// Every ear in a room. The declaration is replaced whole by the next roster;
/// a retuning is one seat's own, and stands until that seat or a reseat writes
/// over it — so the two layers are kept apart and the fold reads them in that
/// order, whatever order the frames arrived in.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Ears {
    declared: BTreeMap<String, Ear>,
    retuned: BTreeMap<String, Ear>,
}

impl Ears {
    /// What a seat hears. The one answer to that question in this crate.
    pub fn of(&self, member: &str) -> Ear {
        let key = key(member);
        self.retuned
            .get(&key)
            .or_else(|| self.declared.get(&key))
            .copied()
            .unwrap_or_default()
    }

    /// The whole of a roster's declaration, from the membership payload it was
    /// published as.
    pub fn declare(&mut self, payload: &Value) {
        self.declared = declared(payload);
    }

    /// One seat's own retuning, as its `ear:` payload has it. A cleared one —
    /// what a reseat writes — leaves the seat with what the roster declared.
    pub fn retune(&mut self, member: &str, payload: &Value) {
        match stored(payload) {
            Some(ear) => self.retuned.insert(key(member), ear),
            None => self.retuned.remove(&key(member)),
        };
    }

    /// The seats a reseat has to clear: a roster declared whole replaces the
    /// ears with it, and a retuning is only cleared where it was written.
    pub fn retuned(&self) -> Vec<String> {
        self.retuned.keys().cloned().collect()
    }
}

/// A name as the ears are keyed: a room compares names in any case, so the
/// key is one spelling of it.
fn key(member: &str) -> String {
    member.to_lowercase()
}

/// The kind one seat's ear is published under. A kind per seat, so two seats
/// retuning at once write two facts rather than racing over one (ADR-0029 §4).
pub fn kind(member: &str) -> String {
    format!("{EAR}{}", key(member))
}

/// What a retuning is published as, and what clears one.
pub fn register(ear: Ear) -> Value {
    json!({ PATIENCE_S: ear.seconds() })
}

/// The listeners a membership payload names. A payload without them — every
/// room opened before there were ears, and every roster whose seats all take
/// the default — declares nothing, so every seat in it wears the default ear.
fn declared(payload: &Value) -> BTreeMap<String, Ear> {
    payload[LISTENERS]
        .as_array()
        .map(|listeners| listeners.iter().filter_map(listed).collect())
        .unwrap_or_default()
}

/// One entry of a payload's `listeners`: a name and the patience beside it.
fn listed(entry: &Value) -> Option<(String, Ear)> {
    let name = entry["name"].as_str()?;
    let ear = match entry[PATIENCE_S].as_u64() {
        Some(seconds) => Ear::stored(seconds),
        None => Ear::default(),
    };
    Some((key(name), ear))
}

/// The ear a register holds, or nothing for one that was cleared.
fn stored(payload: &Value) -> Option<Ear> {
    payload[PATIENCE_S].as_u64().map(Ear::stored)
}

/// The listeners a roster is published with: the seats whose ear is not the
/// default, and nothing at all when none of them differ — a roster of bare
/// names is written exactly as one was before there were ears.
pub fn listeners_of(seats: &[Seat]) -> Option<Value> {
    let listed: Vec<Value> = seats
        .iter()
        .filter(|seat| seat.ear != Ear::default())
        .map(|seat| json!({ "name": seat.name, PATIENCE_S: seat.ear.seconds() }))
        .collect();
    (!listed.is_empty()).then(|| Value::Array(listed))
}

/// Every ear a room's own journal has: what its roster declared, and every
/// retuning published since. Both come from the one snapshot, so neither can
/// drift from the other.
pub fn ears_of(state: &SessionState) -> Ears {
    let mut ears = Ears::default();
    let Some(kinds) = state.extensions.get(PLUGIN) else {
        return ears;
    };
    if let Some(payload) = kinds.get(MEMBERS) {
        ears.declare(payload);
    }
    for (kind, payload) in kinds {
        if let Some(member) = kind.strip_prefix(EAR) {
            ears.retune(member, payload);
        }
    }
    ears
}

