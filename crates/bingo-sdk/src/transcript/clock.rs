//! A turn's working time, derived only from its items and open asks.
use jiff::{SignedDuration, Timestamp};

use crate::{Item, ItemBody, SessionState, TurnId};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TurnClock {
    pub elapsed: SignedDuration,
    /// The next elapsed whole second, absent while waiting or finished.
    pub next_second: Option<Timestamp>,
}

/// Whether a turn is still working, waiting for a person, or over.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Ends {
    Live { asked: Option<Timestamp> },
    Over,
}

impl Ends {
    pub fn of(state: &SessionState, turn: &TurnId) -> Self {
        if state.turn.as_ref().is_some_and(|live| &live.id == turn) {
            Self::Live {
                asked: state
                    .interactions
                    .iter()
                    .filter(|ask| ask.turn.as_ref() == Some(turn))
                    .map(|ask| ask.opened_at)
                    .min(),
            }
        } else {
            Self::Over
        }
    }
}

/// Read the same clock for live, newest and historical turns. An evicted or
/// empty turn has no clock. Old receipts without `asked_at` imply no wait.
pub fn clock(state: &SessionState, turn: &TurnId, now: Timestamp) -> Option<TurnClock> {
    let items: Vec<_> = state
        .transcript
        .items()
        .iter()
        .filter(|item| item.turn.as_ref() == Some(turn))
        .map(AsRef::as_ref)
        .collect();
    clock_of(&items, Ends::of(state, turn), now)
}

/// A call's own wait, still open until it starts or finishes without running.
pub fn call_wait(item: &Item) -> Option<(Timestamp, Option<Timestamp>)> {
    let ItemBody::ToolCall {
        asked_at, began_at, ..
    } = item.body
    else {
        return None;
    };
    Some((asked_at?, began_at.or(item.completed_at)))
}

/// Measure one turn's ordered items, subtracting the union of recorded waits.
pub fn clock_of(items: &[&Item], ends: Ends, now: Timestamp) -> Option<TurnClock> {
    let start = items.first()?.started_at;
    let (end, ticking) = match ends {
        Ends::Live { asked } => {
            let asked = asked
                .into_iter()
                .chain(items.iter().filter_map(|item| {
                    let (asked, end) = call_wait(item)?;
                    end.is_none().then_some(asked)
                }))
                .min();
            (asked.unwrap_or(now).max(start), asked.is_none())
        }
        Ends::Over => (
            items
                .iter()
                .filter_map(|item| item.completed_at)
                .max()
                .unwrap_or(start)
                .max(start),
            false,
        ),
    };
    let elapsed = end.duration_since(start) - waiting(closed_waits(items, start, end));
    Some(TurnClock {
        elapsed,
        next_second: deadline(ticking, now, elapsed),
    })
}

fn closed_waits(items: &[&Item], start: Timestamp, end: Timestamp) -> Vec<(Timestamp, Timestamp)> {
    items
        .iter()
        .filter_map(|item| {
            if let Some((asked, until)) = call_wait(item) {
                return Some((asked.max(start), until.unwrap_or(end).min(end)));
            }
            let asked = match item.body {
                ItemBody::PermissionReceipt { asked_at, .. }
                | ItemBody::QuestionAnswer { asked_at, .. }
                | ItemBody::Interruption { asked_at, .. } => asked_at?,
                _ => return None,
            };
            Some((asked.max(start), item.started_at.min(end)))
        })
        .collect()
}

fn deadline(ticking: bool, now: Timestamp, elapsed: SignedDuration) -> Option<Timestamp> {
    ticking.then(|| next_second(now, elapsed)).flatten()
}

/// When an elapsed duration next reaches a whole second.
pub fn next_second(now: Timestamp, elapsed: SignedDuration) -> Option<Timestamp> {
    let remainder = elapsed.as_nanos() % 1_000_000_000;
    now.checked_add(SignedDuration::from_nanos(
        (1_000_000_000 - remainder) as i64,
    ))
    .ok()
}

fn waiting(mut intervals: Vec<(Timestamp, Timestamp)>) -> SignedDuration {
    intervals.sort_unstable();
    let mut until = Timestamp::MIN;
    let mut total = SignedDuration::ZERO;
    for (start, end) in intervals {
        let start = start.max(until);
        if end > start {
            total += end.duration_since(start);
            until = end;
        }
    }
    total
}
