//! Ordered plugin work for inputs, kept off the actor's mailbox loop.

use std::collections::VecDeque;
use std::panic::AssertUnwindSafe;
use std::sync::Arc;

use bingo_sdk::*;
use futures::FutureExt;
use serde_json::json;

use super::{Actor, commands, inputs, mailbox::Msg, running::supervised};
use crate::turn::HookSet;

pub(super) enum Kind {
    Submit,
    Prose,
    Queued,
    Deliver(Delivery),
}

struct Waiting {
    intent: IntentId,
    input: Input,
    kind: Kind,
}

struct Flight {
    intent: IntentId,
    cancel: CancellationToken,
}

#[derive(Default)]
pub(super) struct InputGate {
    waiting: VecDeque<Waiting>,
    flight: Option<Flight>,
}

impl InputGate {
    pub(super) fn busy(&self) -> bool {
        self.flight.is_some() || !self.waiting.is_empty()
    }

    pub(super) fn interrupt(&self) -> Option<IntentId> {
        let flight = self.flight.as_ref()?;
        flight.cancel.cancel();
        Some(flight.intent.clone())
    }

    fn finish(&mut self, intent: &IntentId) -> Option<bool> {
        if self
            .flight
            .as_ref()
            .is_none_or(|flight| &flight.intent != intent)
        {
            return None;
        }
        self.flight
            .take()
            .map(|flight| flight.cancel.is_cancelled())
    }

    fn drain(&mut self) -> Vec<IntentId> {
        let mut intents = Vec::new();
        if let Some(flight) = self.flight.take() {
            flight.cancel.cancel();
            intents.push(flight.intent);
        }
        intents.extend(self.waiting.drain(..).map(|waiting| waiting.intent));
        intents
    }
}

pub(crate) enum Verdict {
    Prose(Input),
    Command {
        input: Input,
        parsed: commands::Parsed,
        command: Arc<dyn Command>,
        queued: bool,
    },
    Rejected(KernelError),
    Redirected {
        to: SessionId,
        result: Result<(), KernelError>,
    },
    Delivered {
        input: Input,
        delivery: Delivery,
    },
}

impl Actor {
    pub(super) fn gate_input(&mut self, intent: IntentId, input: Input, kind: Kind) {
        let waiting = Waiting {
            intent,
            input,
            kind,
        };
        if matches!(waiting.kind, Kind::Queued | Kind::Prose) {
            self.gate.waiting.push_front(waiting);
        } else {
            self.gate.waiting.push_back(waiting);
        }
        self.start_gate();
    }

    fn start_gate(&mut self) {
        if self.gate.flight.is_some() || self.closing.is_some() || self.state.closed {
            return;
        }
        let Some(waiting) = self.gate.waiting.pop_front() else {
            return;
        };
        let cancel = self.cancel.child_token();
        self.gate.flight = Some(Flight {
            intent: waiting.intent.clone(),
            cancel: cancel.clone(),
        });
        let lookup = self.commands.lookup();
        let hooks = self.config.hooks.clone();
        let cx = self.hook_context();
        let mailbox = self.mailbox.clone();
        self.tracker.spawn(async move {
            let intent = waiting.intent.clone();
            let work = AssertUnwindSafe(evaluate(waiting, lookup, hooks, cx)).catch_unwind();
            let verdict = match supervised(work, &cancel).await {
                _ if cancel.is_cancelled() => Verdict::Rejected(commands::interrupted()),
                Some(Ok(verdict)) => verdict,
                Some(Err(panic)) => Verdict::Rejected(KernelError::new(
                    ErrorCode::Internal,
                    format!("the input gate panicked: {}", super::panic_message(panic)),
                )),
                None => Verdict::Rejected(commands::interrupted()),
            };
            mailbox.send(Msg::Gated { intent, verdict });
        });
    }

    pub(super) async fn gated(
        &mut self,
        intent: IntentId,
        verdict: Verdict,
    ) -> Result<(), KernelError> {
        let Some(cancelled) = self.gate.finish(&intent) else {
            return Ok(());
        };
        let verdict = if cancelled {
            Verdict::Rejected(commands::interrupted())
        } else {
            verdict
        };
        self.apply_gate(intent, verdict).await?;
        self.start_gate();
        self.drain_queue().await
    }

    async fn apply_gate(&mut self, intent: IntentId, verdict: Verdict) -> Result<(), KernelError> {
        match verdict {
            Verdict::Prose(input) => self.submit_prose(intent, input).await,
            Verdict::Command {
                input,
                parsed,
                command,
                queued,
            } => {
                self.submit_command(intent, input, parsed, command, queued)
                    .await
            }
            Verdict::Rejected(error) => self.reject(intent, error.code, error.message).await,
            Verdict::Redirected { to, result } => match result {
                Ok(()) => self.applied(intent, json!({ "redirected": to })).await,
                Err(error) => self.reject(intent, error.code, error.message).await,
            },
            Verdict::Delivered { input, delivery } => {
                self.deliver_ready(intent, input, delivery).await
            }
        }
    }

    pub(super) async fn close_gate(&mut self) -> Result<(), KernelError> {
        for intent in self.gate.drain() {
            self.reject(intent, ErrorCode::SessionClosed, "the session is closed")
                .await?;
        }
        Ok(())
    }
}

async fn evaluate(
    waiting: Waiting,
    lookup: commands::Lookup,
    hooks: HookSet,
    cx: HookContext,
) -> Verdict {
    let Waiting { input, kind, .. } = waiting;
    if let Kind::Deliver(delivery) = kind {
        return Verdict::Delivered { input, delivery };
    }
    if !matches!(kind, Kind::Prose)
        && let Some(parsed) = commands::parse(&input)
    {
        return find_command(input, parsed, lookup, matches!(kind, Kind::Queued)).await;
    }
    if let Err(message) = inputs::validate(&input) {
        return Verdict::Rejected(KernelError::new(ErrorCode::InvalidInput, message));
    }
    submit_hooks(input, hooks, cx).await
}

async fn find_command(
    input: Input,
    parsed: commands::Parsed,
    lookup: commands::Lookup,
    queued: bool,
) -> Verdict {
    if commands::is_shell(&parsed.name) && !commands::may_run_shell(&input) {
        return Verdict::Rejected(KernelError::new(
            ErrorCode::PermissionDenied,
            "a shell line is the person's own: nobody else may run one",
        ));
    }
    match lookup.find(&parsed.name).await {
        Some(command) => Verdict::Command {
            input,
            parsed,
            command,
            queued,
        },
        None => Verdict::Rejected(KernelError::new(
            ErrorCode::InvalidInput,
            commands::unknown(&parsed.name),
        )),
    }
}

async fn submit_hooks(mut input: Input, hooks: HookSet, cx: HookContext) -> Verdict {
    for hook in hooks.at(HookPoint::Submit, None).await {
        match hook.on_submit(&mut input, &cx).await {
            HookOutcome::Continue | HookOutcome::Ask { .. } => {}
            HookOutcome::Deny { reason } | HookOutcome::Block { reason } => {
                return Verdict::Rejected(KernelError::new(ErrorCode::PermissionDenied, reason));
            }
            HookOutcome::Redirect { session } => {
                let result = cx
                    .host
                    .deliver(&session, IntentId::mint(), input, Delivery::Wake)
                    .await;
                return Verdict::Redirected {
                    to: session,
                    result,
                };
            }
        }
    }
    Verdict::Prose(input)
}
