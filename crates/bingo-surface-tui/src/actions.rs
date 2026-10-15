//! UI actions reach the host one at a time, in the order of the gestures.
mod binding;
mod control;
pub use binding::{Binding, Resolution};
use bingo_sdk::{Action, IntentId, Origin, SessionHandle, SessionId};
use std::{
    collections::VecDeque,
    time::{Duration, Instant},
};

/// A broken renderer must not hold unrelated gestures indefinitely. The queue
/// is one line, so a site's wait holds every gesture behind it too.
pub(crate) const REDRAW_WAIT: Duration = Duration::from_millis(250);

#[derive(Debug)]
enum Target {
    Action(Action),
    /// The gesture as drawn, to be re-addressed to the drawing current when
    /// it leaves; until then it is the one a person sees waiting.
    Control(Binding, Action),
}
#[derive(Debug)]
struct Pending {
    session: SessionId,
    handle: SessionHandle,
    intent: IntentId,
    target: Target,
    /// Who signed the gesture when it was made; the drawing it is re-addressed
    /// to later never changes that.
    origin: Option<Origin>,
    sent: bool,
    deadline: Option<Instant>,
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
        (action, origin): (Action, Option<Origin>),
        binding: Option<Binding>,
    ) {
        let target = match binding {
            Some(binding) => Target::Control(binding, action),
            None => Target::Action(action),
        };
        self.pending.push_back(Pending {
            session,
            handle,
            intent,
            target,
            origin,
            sent: false,
            deadline: None,
        });
    }

    pub fn action(&self) -> Option<&Action> {
        match &self.pending.front()?.target {
            Target::Action(action) | Target::Control(_, action) => Some(action),
        }
    }

    pub fn acknowledge(&mut self, intent: &IntentId) {
        if self
            .pending
            .front()
            .is_some_and(|p| p.sent && &p.intent == intent)
        {
            self.pending.pop_front();
        }
    }

    pub fn discard(&mut self, sessions: &[SessionId]) -> Vec<IntentId> {
        let mut discarded = Vec::new();
        self.pending.retain(|pending| {
            if sessions.contains(&pending.session) {
                discarded.push(pending.intent.clone());
                false
            } else {
                true
            }
        });
        discarded
    }

    pub fn deadline(&self) -> Option<Instant> {
        self.pending.front().and_then(|p| p.deadline)
    }

    pub fn release(
        &mut self,
        now: Instant,
        mut resolve: impl FnMut(&SessionId, &Binding) -> Resolution,
    ) -> Vec<(IntentId, String)> {
        let mut dropped = Vec::new();
        while let Some(pending) = self.pending.front_mut().filter(|p| !p.sent) {
            match pending.addressed(now, &mut resolve) {
                Addressed::Held => break,
                Addressed::Stale(notice) => {
                    dropped.push((pending.intent.clone(), notice));
                    self.pending.pop_front();
                }
                Addressed::Ready(action) => {
                    pending.send(action);
                    break;
                }
            }
        }
        dropped
    }
}

enum Addressed {
    Held,
    Stale(String),
    Ready(Action),
}

impl Pending {
    /// What this gesture names now: a held one keeps its first deadline.
    fn addressed(
        &mut self,
        now: Instant,
        resolve: &mut impl FnMut(&SessionId, &Binding) -> Resolution,
    ) -> Addressed {
        let (binding, gesture) = match &self.target {
            Target::Action(action) => return Addressed::Ready(action.clone()),
            Target::Control(binding, gesture) => (binding, gesture),
        };
        let resolution = resolve(&self.session, binding);
        if resolution.waiting && now < *self.deadline.get_or_insert(now + REDRAW_WAIT) {
            return Addressed::Held;
        }
        match resolution
            .action
            .filter(|action| action.name == gesture.name)
        {
            Some(action) => match valued(action, gesture) {
                Some(action) => Addressed::Ready(action),
                None => Addressed::Stale(binding.notice()),
            },
            None => Addressed::Stale(binding.notice()),
        }
    }

    fn send(&mut self, action: Action) {
        let input = bingo_sdk::Input::Action {
            action: action.clone(),
            origin: self.origin.clone(),
        };
        self.handle.submit(self.intent.clone(), input);
        self.target = Target::Action(action);
        self.sent = true;
        self.deadline = None;
    }
}

/// The current handler carries the value the person gave the drawn one.
fn valued(action: Action, gesture: &Action) -> Option<Action> {
    match gesture.args.get("value") {
        Some(value) => action.with_value(value.clone()),
        None => Some(action),
    }
}

#[cfg(test)]
mod value_tests {
    use super::*;
    use serde_json::json;
    #[test]
    fn values_keep_existing_arguments_and_accept_null() {
        for args in [json!(null), json!({"handler":"minted"})] {
            let action = Action {
                name: "probe:ui".into(),
                args: args.clone(),
            };
            let gesture = Action {
                name: "probe:ui".into(),
                args: json!({"value":"typed"}),
            };
            let changed = valued(action, &gesture).unwrap();
            assert_eq!(changed.args["value"], "typed");
            assert_eq!(changed.args["handler"], args["handler"]);
        }
    }
    #[test]
    fn malformed_value_arguments_do_not_panic() {
        for args in [json!(false), json!(42), json!("text"), json!([])] {
            let action = Action {
                name: "probe:ui".into(),
                args,
            };
            let gesture = Action {
                name: "probe:ui".into(),
                args: json!({"value":"typed"}),
            };
            assert!(valued(action, &gesture).is_none());
        }
    }
}
