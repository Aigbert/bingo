//! A round walks once; each decision belongs to the call it has reached.

use super::*;
use crate::executor::{Dispatch, InFlight};
use crate::gate::{self, Decided, Gated};
use crate::round::{Action, next};

pub(super) async fn run(turn: &mut Turn<'_>, finished: &Finished) -> Vec<ToolOutcome> {
    let mut outcomes = Vec::with_capacity(finished.tool_calls.len());
    let report = crate::chain::Report::default();
    let links = turn.hooks.links.clone();
    let cx = turn.hook_cx.clone();
    let dispatch = Dispatch {
        links: &links,
        cx: &cx,
        report: report.clone(),
    };
    let mut flight = InFlight::default();
    if let Some(hooks) = turn.racing(turn.hooks.gather()).await {
        walk_calls(
            turn,
            finished,
            &hooks,
            &dispatch,
            &mut flight,
            &mut outcomes,
        )
        .await;
    }
    if turn.cancel.is_cancelled() {
        for outcome in flight.cancelled() {
            land(turn, outcome, &mut outcomes);
        }
    }
    interrupt_remaining(turn, finished, &mut outcomes);
    for notice in report.take() {
        turn.record(notice);
    }
    outcomes.sort_by_key(|outcome| {
        finished
            .tool_calls
            .iter()
            .position(|(item, _)| item == &outcome.item)
    });
    outcomes
}

async fn walk_calls<'a>(
    turn: &mut Turn<'_>,
    finished: &Finished,
    hooks: &[Arc<dyn Hook>],
    dispatch: &Dispatch<'a>,
    flight: &mut InFlight<'a>,
    outcomes: &mut Vec<ToolOutcome>,
) -> Option<()> {
    for (item, call) in &finished.tool_calls {
        while !flight.state().all_safe {
            land_next(turn, flight, outcomes).await?;
        }
        let cfg = turn.cfg;
        let tool = turn
            .late
            .tools
            .iter()
            .find(|tool| tool.spec().name == call.name)
            .cloned();
        let input = GateInput {
            session: &cfg.session.id,
            cwd: &cfg.cwd,
            item,
            call: call.clone(),
            tool: tool.clone(),
            policy: cfg.policy.as_ref(),
            hooks,
            links: dispatch.links,
            hook_cx: dispatch.cx,
        };
        let decided = progressing(turn, flight, outcomes, gate::decide(&input)).await?;
        let gated = settle_reached(turn, input, decided, flight, outcomes).await?;
        let call = admit(turn, item, gated, tool).await?;
        let cx = tool_context(turn, &call);
        if let Some(outcome) = flight.start(call, cx, turn.cancel.clone(), Some(dispatch.clone())) {
            land(turn, outcome, outcomes);
        }
    }
    while next(flight.state(), None) == Action::Wait {
        land_next(turn, flight, outcomes).await?;
    }
    Some(())
}

async fn admit(
    turn: &mut Turn<'_>,
    item: &ItemId,
    mut gated: Gated,
    tool: Option<Arc<dyn Tool>>,
) -> Option<PendingCall> {
    for notice in gated.notices.drain(..) {
        turn.record(notice);
    }
    if !turn.record_receipt(gated.receipt.take()).await || turn.cancel.is_cancelled() {
        return None;
    }
    mark_gated(turn, item, &mut gated);
    Some(PendingCall {
        item: item.clone(),
        call: gated.call,
        tool,
        traits: gated.traits,
        gate: gated.gate,
    })
}

async fn settle_reached(
    turn: &mut Turn<'_>,
    input: GateInput<'_>,
    decided: Decided,
    flight: &mut InFlight<'_>,
    outcomes: &mut Vec<ToolOutcome>,
) -> Option<Gated> {
    while next(flight.state(), Some(decided.action())) == Action::Wait {
        land_next(turn, flight, outcomes).await?;
    }
    match decided {
        Decided::Settled(gated) => Some(*gated),
        decided @ Decided::Ask(_) => {
            let prompter = AskVia {
                host: turn.host,
                item: input.item.clone(),
            };
            turn.racing(gate::ask(input, decided, &prompter)).await
        }
    }
}

/// Earlier safe calls keep progressing while a reached call is deciding.
async fn progressing<T>(
    turn: &mut Turn<'_>,
    flight: &mut InFlight<'_>,
    outcomes: &mut Vec<ToolOutcome>,
    work: impl Future<Output = T>,
) -> Option<T> {
    tokio::pin!(work);
    let cancel = turn.cancel.clone();
    racing(&cancel, async {
        loop {
            tokio::select! {
                biased;
                Some(outcome) = flight.next(), if flight.state().count > 0 => {
                    land(turn, outcome, outcomes);
                }
                result = &mut work => return result,
            }
        }
    })
    .await
}

async fn land_next(
    turn: &mut Turn<'_>,
    flight: &mut InFlight<'_>,
    outcomes: &mut Vec<ToolOutcome>,
) -> Option<()> {
    let outcome = turn.racing(flight.next()).await??;
    land(turn, outcome, outcomes);
    Some(())
}

fn tool_context(turn: &Turn<'_>, call: &PendingCall) -> ToolContext {
    ToolContext {
        call_id: call.call.call_id.clone(),
        session: turn.cfg.session.id.clone(),
        turn: turn.id.clone(),
        item: call.item.clone(),
        cwd: turn.cfg.cwd.clone(),
        cancel: turn.cancel.child_token(),
        env: turn.cfg.env.clone(),
        host: turn.cfg.host.clone(),
        call: turn.cfg.call_host.tool.clone(),
    }
}

fn interrupt_remaining(turn: &mut Turn<'_>, finished: &Finished, outcomes: &mut Vec<ToolOutcome>) {
    for (item, call) in &finished.tool_calls {
        if outcomes.iter().any(|outcome| &outcome.item == item) {
            continue;
        }
        let pending = PendingCall {
            item: item.clone(),
            call: call.clone(),
            tool: None,
            traits: ToolTraits::default(),
            gate: Gate::Allowed,
        };
        land(turn, executor::interrupted(&pending), outcomes);
    }
}

fn land(turn: &mut Turn<'_>, outcome: ToolOutcome, outcomes: &mut Vec<ToolOutcome>) {
    if let Some(mut item) = turn.transcript.get(&outcome.item).cloned() {
        item.status = outcome.status;
        item.completed_at = Some(Timestamp::now());
        if let ItemBody::ToolCall {
            output,
            duration_ms,
            progress,
            ..
        } = &mut item.body
        {
            *output = Some(outcome.output.clone());
            *duration_ms = Some(outcome.duration_ms);
            *progress = None;
        }
        emit_and_fold(
            &mut turn.transcript,
            turn.host,
            Event::ItemCompleted { item },
        );
    }
    outcomes.push(outcome);
}

fn mark_gated(turn: &mut Turn<'_>, item_id: &ItemId, gated: &mut crate::gate::Gated) {
    if let Some(mut item) = turn.transcript.get(item_id).cloned()
        && (gated.gate == Gate::Allowed
            || matches!(&item.body, ItemBody::ToolCall { input, .. } if *input != gated.call.input))
    {
        if gated.gate == Gate::Allowed {
            item.status = ItemStatus::Running;
        }
        if let ItemBody::ToolCall {
            input, allowed_by, ..
        } = &mut item.body
        {
            *input = gated.call.input.clone();
            *allowed_by = gated.allowed_by.take().map(Box::new);
        }
        emit_and_fold(&mut turn.transcript, turn.host, Event::ItemUpdated { item });
    }
}
