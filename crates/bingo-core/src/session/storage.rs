//! A journal failure ends the live segment without pretending its tail is durable.

use bingo_sdk::*;
use jiff::Timestamp;

use super::Actor;

impl Actor {
    pub(super) fn storage_failed(&mut self, error: &KernelError) {
        tracing::error!(session = %self.id, %error, "journal append failed");
        self.rx.close();
        if let Some(running) = self.running.take() {
            running.abandon();
        }
        self.terminal_event(
            Event::Notice {
                level: Level::Error,
                code: "STORAGE".into(),
                text: error.message.clone(),
            },
            None,
        );
        for (intent, _) in self.queue.drain_all() {
            self.terminal_event(
                Event::IntentAck {
                    intent: intent.clone(),
                    outcome: IntentOutcome::Rejected {
                        error: error.clone(),
                    },
                },
                Some(intent),
            );
        }
        for (_, pending) in self.pending.drain() {
            let _ = pending.reply.send(Err(error.clone()));
        }
        self.terminal_event(
            Event::SessionClosed {
                reason: CloseReason::Error {
                    message: error.message.clone(),
                },
            },
            None,
        );
        self.subscribers.clear();
    }

    fn terminal_event(&mut self, event: Event, cause: Option<IntentId>) {
        self.seq = self.seq.next();
        self.broadcast(Frame {
            seq: self.seq,
            ts: Timestamp::now(),
            session: self.id.clone(),
            cause,
            event,
        });
    }
}
