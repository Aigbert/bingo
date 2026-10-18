//! Close the unfinished segment of a recovered journal.

use bingo_sdk::*;
use jiff::Timestamp;

/// Lost questions and a lost turn close before the resumed session does new work.
pub(super) fn frames(
    interactions: &[Interaction],
    turn: Option<&LiveTurn>,
    transcript: &Transcript,
    at: Timestamp,
) -> Vec<Event> {
    let mut frames: Vec<_> = interactions
        .iter()
        .map(|ask| Event::InteractionCancelled {
            id: ask.id.clone(),
            reason: CancelReason::SessionClosed,
        })
        .collect();
    if let Some(turn) = turn {
        frames.extend(interrupt_items(transcript.open_items(&turn.id), at));
        frames.push(Event::TurnCompleted {
            turn: turn.id.clone(),
            status: TurnStatus::Failed {
                error: KernelError::new(ErrorCode::TurnLost, "the process ended during this turn"),
            },
            usage: turn.usage,
        });
    }
    frames
}

/// Close every open item, including unfinished text, at the supplied instant.
pub(super) fn interrupt_items<'a>(
    open: impl Iterator<Item = &'a Item>,
    at: Timestamp,
) -> Vec<Event> {
    open.map(|item| {
        let mut item = item.clone();
        item.status = ItemStatus::Interrupted;
        item.completed_at = Some(at);
        Event::ItemCompleted { item }
    })
    .collect()
}

#[cfg(test)]
mod tests;
