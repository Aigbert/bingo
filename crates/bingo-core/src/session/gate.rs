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
    calling: Vec<invocation::CallingHook>,
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

pub(super) fn gate_input(actor: &mut Actor, intent: IntentId, input: Input, kind: Kind) {
    let waiting = Waiting {
        calling: invocation::current(),
        intent,
        input,
        kind,
    };
    if matches!(waiting.kind, Kind::Queued | Kind::Prose) {
        actor.gate.waiting.push_front(waiting);
    } else {
        actor.gate.waiting.push_back(waiting);
    }
    start_gate(actor);
}

fn start_gate(actor: &mut Actor) {
    if actor.gate.flight.is_some() || actor.closing.is_some() || actor.state.closed {
        return;
    }
    let Some(waiting) = actor.gate.waiting.pop_front() else {
        return;
    };
    let cancel = actor.cancel.child_token();
    actor.gate.flight = Some(Flight {
        intent: waiting.intent.clone(),
        cancel: cancel.clone(),
    });
    let lookup = actor.commands.lookup();
    let hooks = actor.config.hooks.clone();
    let cx = actor.hook_context();
    let mailbox = actor.mailbox.clone();
    let calling = waiting.calling.clone();
    actor.tracker.spawn(invocation::scope(calling, async move {
        let intent = waiting.intent.clone();
        let report = crate::chain::Report::default();
        let work =
            AssertUnwindSafe(evaluate(waiting, lookup, hooks, cx, report.clone())).catch_unwind();
        let verdict = match supervised(work, &cancel).await {
            _ if cancel.is_cancelled() => Verdict::Rejected(commands::interrupted()),
            Some(Ok(verdict)) => verdict,
            Some(Err(panic)) => Verdict::Rejected(KernelError::new(
                ErrorCode::Internal,
                format!("the input gate panicked: {}", super::panic_message(panic)),
            )),
            None => Verdict::Rejected(commands::interrupted()),
        };
        for notice in report.take() {
            let _ = mailbox.record(notice).await;
        }
        mailbox.send(Msg::Gated { intent, verdict });
    }));
}

pub(super) async fn gated(
    actor: &mut Actor,
    intent: IntentId,
    verdict: Verdict,
) -> Result<(), KernelError> {
    let Some(cancelled) = actor.gate.finish(&intent) else {
        return Ok(());
    };
    let verdict = if cancelled {
        Verdict::Rejected(commands::interrupted())
    } else {
        verdict
    };
    let held_delivery = matches!(
        verdict,
        Verdict::Delivered {
            delivery: Delivery::Hold,
            ..
        }
    );
    apply_gate(actor, intent, verdict).await?;
    start_gate(actor);
    if held_delivery && !actor.queue.ready() {
        return Ok(());
    }
    actor.drain_queue().await
}

async fn apply_gate(
    actor: &mut Actor,
    intent: IntentId,
    verdict: Verdict,
) -> Result<(), KernelError> {
    match verdict {
        Verdict::Prose(input) => actor.submit_prose(intent, input).await,
        Verdict::Command {
            input,
            parsed,
            command,
            queued,
        } => {
            actor
                .submit_command(intent, input, parsed, command, queued)
                .await
        }
        Verdict::Rejected(error) => actor.reject(intent, error.code, error.message).await,
        Verdict::Redirected { to, result } => match result {
            Ok(()) => actor.applied(intent, json!({ "redirected": to })).await,
            Err(error) => actor.reject(intent, error.code, error.message).await,
        },
        Verdict::Delivered { input, delivery } => {
            actor.deliver_ready(intent, input, delivery).await
        }
    }
}

pub(super) async fn close_gate(actor: &mut Actor) -> Result<(), KernelError> {
    for intent in actor.gate.drain() {
        actor
            .reject(intent, ErrorCode::SessionClosed, "the session is closed")
            .await?;
    }
    Ok(())
}

async fn evaluate(
    waiting: Waiting,
    lookup: commands::Lookup,
    hooks: HookSet,
    cx: HookContext,
    report: crate::chain::Report,
) -> Verdict {
    let Waiting { input, kind, .. } = waiting;
    if let Kind::Deliver(delivery) = kind {
        return match crate::chain::deliver::receive(input, &hooks.links().await, &cx, report).await
        {
            Ok(input) => Verdict::Delivered { input, delivery },
            Err(error) => Verdict::Rejected(error),
        };
    }
    if !matches!(kind, Kind::Prose)
        && let Some(parsed) = commands::parse(&input)
    {
        return find_command(input, parsed, lookup, matches!(kind, Kind::Queued)).await;
    }
    if let Err(message) = inputs::validate(&input) {
        return Verdict::Rejected(KernelError::new(ErrorCode::InvalidInput, message));
    }
    submit_hooks(input, hooks, cx, report).await
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

async fn submit_hooks(
    input: Input,
    hooks: HookSet,
    cx: HookContext,
    report: crate::chain::Report,
) -> Verdict {
    let matched = hooks.at(HookPoint::Submit, None).await;
    let result = crate::chain::run::<crate::chain::submit::Submit>(
        &hooks.links().await,
        Next::new(|input| Box::pin(crate::chain::submit::kernel(input, &matched, &cx))),
        &cx,
        input,
        report,
    )
    .await;
    match result {
        Ok(SubmitResult::Accepted(input)) => Verdict::Prose(input),
        Ok(SubmitResult::Rejected(error)) | Err(error) => Verdict::Rejected(error),
        Ok(SubmitResult::Redirect { session, input }) => {
            let result = cx
                .host
                .deliver(&session, IntentId::mint(), input, Delivery::Wake)
                .await;
            Verdict::Redirected {
                to: session,
                result,
            }
        }
    }
}

#[cfg(test)]
#[path = "tests/hook_contract.rs"]
mod hook_contract;
