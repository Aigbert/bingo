//! Room facts used to navigate a surface, including legacy card readings.
use super::{EAR, LISTENERS, MEMBERS, PLUGIN};
use crate::name::same;
use bingo_sdk::{ItemBody, SessionState};
use jiff::Timestamp;
use serde_json::Value;

/// The signal the room's parent carries while any answer is owed.
const OWED: &str = "owed";
/// The open debts that signal carries beside the table it draws as, and the
/// fields of one: which room it stands in, who has not answered, and the
/// moment it was asked (ADR-0022 §4).
const DEBTS: &str = "debts";
const ROOM: &str = "room";
const WHO: &str = "who";
const AT: &str = "at";
/// How long a post waits for a patient seat, in both payloads that say it.
const PATIENCE_S: &str = "patience_s";
/// The columns of the `owed` table, by the headers it publishes them under.
const ROOM_COLUMN: &str = "room";
const OWED_COLUMN: &str = "owed";
const ASKED_COLUMN: &str = "asked";

/// How a seat hears its room (ADR-0029 §1, reversed by ADR-0034 §6).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Hearing {
    /// Every post wakes it as it lands. The exception now, asked for by the
    /// number that says so — `name:0` on a roster.
    Live,
    /// Posts land held and are read whole at the seat's next turn. The
    /// patience is absent where the roster named the seat without one, which
    /// is what a bare name asks for and what nearly every seat wears.
    Listening { patience_s: Option<u64> },
}

/// A bare name on a roster is a patient ear (ADR-0034 §6). How long it waits
/// is the plugin's own constant and is published only where a seat asked for
/// another, so a seat the roster says nothing about listens for a while this
/// surface has no number for.
impl Default for Hearing {
    fn default() -> Hearing {
        Hearing::Listening { patience_s: None }
    }
}

/// An answer a seat still owes, as the card it is read from says it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Owes {
    /// The moment the question was put (`debts[].at`). How long ago that was
    /// is the row's to say, from the clock the frame is drawn against.
    Since(Timestamp),
    /// The clock time a card published before the debts carried their own
    /// stamps says — `14:02`, with no date beside it. An age would have to
    /// invent the day and the zone it was written in, so that row says the
    /// time it was asked at and nothing more.
    At(String),
}

/// What stands unread in the room for this seat: the posts of the room's own
/// journal that landed after the one its mark points at. A seat with no mark,
/// a mark of a shape this surface does not know, or one naming a post the
/// journal no longer holds is measured against nothing, and says nothing.
pub fn unread_count(room: &SessionState, name: &str) -> Option<u64> {
    let read = read_to(room, name)?;
    let posts = posts(room);
    let at = posts.iter().position(|post| *post == read)?;
    let behind = (posts.len() - at - 1) as u64;
    (behind > 0).then_some(behind)
}

/// The post this seat's mark stopped at, as the room's own journal has it.
fn read_to<'a>(room: &'a SessionState, name: &str) -> Option<&'a str> {
    published(room, &crate::cursor::kind(name))?
        .get("post")?
        .as_str()
}

/// The posts a room holds, in the order its journal gave them. A post is what
/// somebody said into the room, which is a user item — the same thing the
/// rooms plugin folds its own ledgers out of.
fn posts(room: &SessionState) -> Vec<&str> {
    room.transcript
        .items()
        .iter()
        .filter(|item| matches!(item.body, ItemBody::User { .. }))
        .map(|item| item.id.as_str())
        .collect()
}

/// What one seat hears: its own retuning where it has written one, else what
/// the roster declared for it — the two layers ADR-0029 §4 keeps apart, read
/// in that order.
pub fn hearing(room: &SessionState, name: &str) -> Hearing {
    retuned(room, name).unwrap_or_else(|| declared(room, name))
}

/// The seat's own `ear:<name>` register. A patience of zero is a live ear
/// said the long way round.
fn retuned(room: &SessionState, name: &str) -> Option<Hearing> {
    let seconds = published(room, &format!("{EAR}{}", name.to_lowercase()))?
        .get(PATIENCE_S)?
        .as_u64()?;
    Some(listening(Some(seconds)))
}

/// The ear the roster was seated with. A roster names only the seats whose ear
/// is not the default, so a seat it passes over — a bare name, and every name
/// on a room opened before there were ears — wears that default.
fn declared(room: &SessionState, name: &str) -> Hearing {
    published(room, MEMBERS)
        .and_then(|payload| payload.get(LISTENERS).cloned())
        .as_ref()
        .and_then(Value::as_array)
        .and_then(|listeners| {
            listeners
                .iter()
                .find(|listener| names(listener, name))
                .cloned()
        })
        .map(|listener| listening(listener.get(PATIENCE_S).and_then(Value::as_u64)))
        .unwrap_or_default()
}

/// A patience as an ear: zero seconds is the live seat it describes.
fn listening(patience_s: Option<u64>) -> Hearing {
    match patience_s {
        Some(0) => Hearing::Live,
        patience_s => Hearing::Listening { patience_s },
    }
}

/// Whether a listener names this seat. A door takes a bare name or a name with
/// the patience it asks for, and both shapes reach the journal.
fn names(listener: &Value, name: &str) -> bool {
    let said = listener
        .as_str()
        .or_else(|| listener.get("name").and_then(Value::as_str));
    said.is_some_and(|said| same(said, name))
}

/// Debts published on a room's parent, with the oldest first.
pub fn debts(parent: &SessionState, title: &str) -> Vec<(String, Owes)> {
    let Some(card) = live(parent, OWED) else {
        return Vec::new();
    };
    facts(&card, title).unwrap_or_else(|| table(&card, title))
}

/// The debts the card carries beside the table it draws as: the moment each
/// question was put, which is what lets a row say how long it has stood.
/// `None` where the payload has no debts at all — a card left by a process
/// that published only the table, whose clock time the fallback reads.
fn facts(card: &Value, title: &str) -> Option<Vec<(String, Owes)>> {
    let debts = card.get(DEBTS)?.as_array()?;
    let said = |debt: &Value, field: &str| debt.get(field)?.as_str().map(str::to_string);
    Some(
        debts
            .iter()
            .filter(|debt| said(debt, ROOM).is_some_and(|named| same(&named, title)))
            .filter_map(|debt| Some((said(debt, WHO)?, Owes::Since(asked(debt)?))))
            .collect(),
    )
}

/// The moment a debt was asked, as the payload states it: RFC 3339, the
/// spelling every timestamp on the wire wears.
fn asked(debt: &Value) -> Option<Timestamp> {
    debt.get(AT)?.as_str()?.parse().ok()
}

/// The `owed` table, read by its own headers rather than by the order its
/// columns happen to be in: it is a `View::Table` a plugin wrote, not a shape
/// this surface gets to assume. Only a card without debts is read this far,
/// and only such a card has the `asked` column this wants.
fn table(card: &Value, title: &str) -> Vec<(String, Owes)> {
    let column = |header: &str| {
        card.get("headers")?
            .as_array()?
            .iter()
            .position(|cell| cell.as_str() == Some(header))
    };
    let (Some(room), Some(who), Some(asked)) = (
        column(ROOM_COLUMN),
        column(OWED_COLUMN),
        column(ASKED_COLUMN),
    ) else {
        return Vec::new();
    };
    let cell = |row: &Value, at: usize| row.get(at)?.as_str().map(str::to_string);
    card.get("rows")
        .and_then(Value::as_array)
        .map(|rows| {
            rows.iter()
                .filter(|row| cell(row, room).is_some_and(|named| same(&named, title)))
                .filter_map(|row| Some((cell(row, who)?, Owes::At(cell(row, asked)?))))
                .collect()
        })
        .unwrap_or_default()
}

/// The whole of one kind a plugin journaled into a session (ADR-0013 §2).
fn published<'a>(state: &'a SessionState, kind: &str) -> Option<&'a Value> {
    state.extensions.get(PLUGIN)?.get(kind)
}

/// The whole of one kind a plugin is signalling onto a session's stream. It
/// is named for the lane rather than the call, so the fixture that writes one
/// keeps the plugin's own word for it.
fn live(state: &SessionState, kind: &str) -> Option<Value> {
    state.signals.get(PLUGIN)?.get(kind).cloned()
}
