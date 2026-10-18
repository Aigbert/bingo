//! A journal failure ends the live segment without pretending its tail is durable.

use bingo_sdk::*;
use jiff::Timestamp;

use super::Actor;

pub(super) fn storage_failed(actor: &mut Actor, error: &KernelError) {
    tracing::error!(session = %actor.id, %error, "journal append failed");
    actor.rx.close();
    actor.remember_stopped_ask();
    if let Some(running) = actor.running.take() {
        running.abandon();
    }
    terminal_event(
        actor,
        Event::Notice {
            level: Level::Error,
            code: "STORAGE".into(),
            text: error.message.clone(),
        },
        None,
    );
    for (intent, _) in actor.queue.drain_all() {
        terminal_event(
            actor,
            Event::IntentAck {
                intent: intent.clone(),
                outcome: IntentOutcome::Rejected {
                    error: error.clone(),
                },
            },
            Some(intent),
        );
    }
    for (_, pending) in actor.pending.drain() {
        let _ = pending.reply.send(Err(error.clone()));
    }
    terminal_event(
        actor,
        Event::SessionClosed {
            reason: CloseReason::Error {
                message: error.message.clone(),
            },
        },
        None,
    );
    actor.subscribers.clear();
}

fn terminal_event(actor: &mut Actor, event: Event, cause: Option<IntentId>) {
    actor.seq = actor.seq.next();
    actor.broadcast(Frame {
        seq: actor.seq,
        ts: Timestamp::now(),
        session: actor.id.clone(),
        cause,
        event,
    });
}
