//! UI actions reach the host one at a time, in the order of the gestures.
use bingo_sdk::{Action, Input, IntentId, SessionHandle, SessionId};
use std::collections::VecDeque;

#[derive(Debug)]
struct Pending {
    session: SessionId,
    handle: SessionHandle,
    intent: IntentId,
    action: Action,
}

#[derive(Debug, Default)]
pub struct Queue {
    pending: VecDeque<Pending>,
}

impl Queue {
    pub fn push(
        &mut self,
        session: SessionId,
        handle: SessionHandle,
        intent: IntentId,
        action: Action,
    ) {
        self.pending.push_back(Pending {
            session,
            handle,
            intent,
            action,
        });
        if self.pending.len() == 1 {
            self.send();
        }
    }

    pub fn action(&self) -> Option<&Action> {
        self.pending.front().map(|pending| &pending.action)
    }

    pub fn acknowledge(&mut self, intent: &IntentId) {
        if self.pending.front().is_some_and(|p| &p.intent == intent) {
            self.pending.pop_front();
            self.send();
        }
    }

    pub fn discard(&mut self, sessions: &[SessionId]) -> Vec<IntentId> {
        let active = self.pending.front().map(|p| p.intent.clone());
        let mut discarded = Vec::new();
        self.pending.retain(|pending| {
            if sessions.contains(&pending.session) {
                discarded.push(pending.intent.clone());
                false
            } else {
                true
            }
        });
        if active.as_ref() != self.pending.front().map(|p| &p.intent) {
            self.send();
        }
        discarded
    }

    fn send(&self) {
        if let Some(pending) = self.pending.front() {
            // Instant commands can finish out of order; an older input must
            // never overwrite a newer one, nor may close overtake an edit.
            pending.handle.submit(
                pending.intent.clone(),
                Input::Action {
                    action: pending.action.clone(),
                },
            );
        }
    }
}
