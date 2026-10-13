//! Showing what is owed (ADR-0022 §4): the column `/room` gains, and the card
//! the room's parent carries while any debt stands. Both are drawn from the
//! fold at the moment they are asked for, and nothing here is kept.

use bingo_sdk::{View};
use jiff::Timestamp;
use serde::Serialize;
use serde_json::{Value, json};

use crate::mentions::Mention;

/// The kind the card is published under. The latest payload is the whole of
/// it, and `Null` is what takes the card away (ADR-0013 §2).
pub const KIND: &str = "owed";

/// The facts the card rides with, beside the table it draws as.
const DEBTS: &str = "debts";

const HEADERS: [&str; 2] = ["room", "owed"];

/// One open debt, as the card carries it: the room it stands in, who has not
/// answered, and the moment it was asked. A reader that knows this signal
/// takes the age it wants from `at`; the card itself says none, because a
/// signal republished only when a debt opens or closes cannot keep one true.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Debt {
    pub room: String,
    pub who: String,
    pub at: Timestamp,
}

/// The `/room` cell: who owes, oldest first, and how long they have owed it.
/// A member with two debts is named once, for the older.
pub fn column(open: &[Mention], now: Timestamp) -> String {
    let mut said: Vec<String> = Vec::new();
    let mut seen: Vec<String> = Vec::new();
    for mention in oldest_first(open) {
        let who = mention.owed_by.said();
        if seen.contains(&who) {
            continue;
        }
        said.push(format!("{who} {}", age(mention.at, now)));
        seen.push(who);
    }
    said.join(", ")
}

/// What one room owes, oldest first: the one mint, from the one fold. A member
/// with two debts owes twice here, and `column` is where two become one.
pub fn debts(title: &str, open: &[Mention]) -> Vec<Debt> {
    oldest_first(open)
        .into_iter()
        .map(|mention| Debt {
            room: title.to_string(),
            who: mention.owed_by.said(),
            at: mention.at,
        })
        .collect()
}

/// The card: a table while anything is owed, and nothing at all once the last
/// debt closes. The debts ride in the same payload as the table drawn from
/// them, the way a roster does (`room::payload`, ADR-0013 §2) — a surface that
/// knows only the vocabulary draws the table, and one that knows this signal
/// reads the facts, without either having to know about the other.
pub fn view(debts: Vec<Debt>) -> Value {
    if debts.is_empty() {
        return Value::Null;
    }
    let mut payload = drawn(&debts);
    payload[DEBTS] = json!(debts);
    payload
}

/// The two columns a person reads on the card: which room, and who has not
/// answered. The clock left it on 2026-09-02 — three columns were one wider
/// than the rail, and the age a person wants is the session list's to say from
/// `at`, drawn as it is asked for rather than as it was published.
fn drawn(debts: &[Debt]) -> Value {
    let view = View::Table {
        headers: HEADERS.map(str::to_string).to_vec(),
        rows: debts
            .iter()
            .map(|debt| vec![debt.room.clone(), debt.who.clone()])
            .collect(),
    };
    serde_json::to_value(view).unwrap_or_default()
}

fn oldest_first(open: &[Mention]) -> Vec<&Mention> {
    let mut sorted: Vec<&Mention> = open.iter().collect();
    sorted.sort_by_key(|mention| mention.at);
    sorted
}

/// How long a debt has stood, as a person says it.
pub fn age(at: Timestamp, now: Timestamp) -> String {
    let seconds = now.duration_since(at).as_secs().max(0);
    match seconds {
        0..=59 => format!("{seconds}s"),
        60..=3599 => format!("{}m", seconds / 60),
        _ => format!("{}h", seconds / 3600),
    }
}

