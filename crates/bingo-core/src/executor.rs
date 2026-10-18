//! Calls in flight: each landing is yielded at once. Admission and result
//! ordering belong to the round; an interrupt drops every unfinished call.
//!
//! One `esc` is the whole of it. Dropping the future *is* the interrupt: a
//! `Bash` call takes its process group down with it, an MCP or plugin bridge
//! abandons the request it was waiting on. A tool has no say in this — a turn
//! a person stopped must stop, and a call that had already written to the
//! working tree is not made safe by waiting for it to write more.

use std::sync::Arc;
use std::time::Instant;

use bingo_sdk::*;
use futures::{FutureExt, StreamExt, future::BoxFuture, stream::FuturesUnordered};

mod asking;

pub const MAX_CONCURRENCY: usize = 10;
pub const MAX_RESULT_CHARS: usize = 50_000;
pub const INTERRUPTED_MARKER: &str = "[Request interrupted by user for tool use]";

/// Whether the gate let the call through.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Gate {
    Allowed,
    Denied { message: String },
}

pub struct PendingCall {
    pub item: ItemId,
    pub call: ToolCall,
    pub tool: Option<Arc<dyn Tool>>,
    pub traits: ToolTraits,
    pub gate: Gate,
}

impl std::fmt::Debug for PendingCall {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PendingCall")
            .field("item", &self.item)
            .field("call", &self.call)
            .field("gate", &self.gate)
            .finish_non_exhaustive()
    }
}

#[derive(Clone)]
pub struct Dispatch<'a> {
    pub links: &'a [crate::chain::Link],
    pub cx: &'a HookContext,
    pub report: crate::chain::Report,
}

struct RunningCall<'a> {
    safe: bool,
    work: BoxFuture<'a, ToolOutcome>,
}

impl Future for RunningCall<'_> {
    type Output = ToolOutcome;

    fn poll(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Self::Output> {
        self.work.as_mut().poll(cx)
    }
}

#[derive(Default)]
pub(crate) struct InFlight<'a>(FuturesUnordered<RunningCall<'a>>);

impl<'a> InFlight<'a> {
    pub(crate) fn state(&self) -> crate::round::Flight {
        crate::round::Flight {
            count: self.0.len(),
            all_safe: self.0.iter().all(|call| call.safe),
        }
    }

    /// Poll at admission: the next call is reached only after this one starts.
    pub(crate) fn start(
        &mut self,
        call: PendingCall,
        cx: ToolContext,
        cancel: CancellationToken,
        dispatch: Option<Dispatch<'a>>,
    ) -> Option<ToolOutcome> {
        let safe = call.traits.concurrency_safe;
        let mut work =
            Box::pin(async move { run_one(&call, cx, &cancel, dispatch.as_ref()).await });
        if let Some(outcome) = work.as_mut().now_or_never() {
            return Some(outcome);
        }
        self.0.push(RunningCall { safe, work });
        None
    }

    /// The token is cancelled: run_one's first branch is ready without polling a tool.
    pub(crate) fn cancelled(self) -> impl Iterator<Item = ToolOutcome> + 'a {
        self.0.into_iter().filter_map(FutureExt::now_or_never)
    }

    pub(crate) async fn next(&mut self) -> Option<ToolOutcome> {
        self.0.next().await
    }
}

pub(crate) fn interrupted(pc: &PendingCall) -> ToolOutcome {
    ToolOutcome {
        item: pc.item.clone(),
        call_id: pc.call.call_id.clone(),
        output: ToolOutput::error(INTERRUPTED_MARKER),
        status: ItemStatus::Interrupted,
        duration_ms: 0,
    }
}

pub(crate) fn tool_not_found(name: &str) -> String {
    format!("tool not found: {name}")
}

async fn run_one(
    pc: &PendingCall,
    cx: ToolContext,
    cancel: &CancellationToken,
    dispatch: Option<&Dispatch<'_>>,
) -> ToolOutcome {
    let cx = asking::bind(cx);
    let started = Instant::now();
    let finish = |output: ToolOutput, status: ItemStatus| ToolOutcome {
        item: pc.item.clone(),
        call_id: pc.call.call_id.clone(),
        output,
        status,
        duration_ms: started.elapsed().as_millis() as u64,
    };
    if let Gate::Denied { message } = &pc.gate {
        return finish(ToolOutput::error(message.clone()), ItemStatus::Failed);
    }
    let Some(tool) = &pc.tool else {
        return finish(
            ToolOutput::error(tool_not_found(&pc.call.name)),
            ItemStatus::Failed,
        );
    };
    // Biased, cancel first: once a person has stopped the turn the call is
    // never polled again, so what ends it is always the drop and never a
    // tool's own reading of its token — two ways to end one call would be two
    // outcomes for one keypress.
    let result = tokio::select! {
        biased;
        _ = cancel.cancelled() => Err(ToolError::Cancelled),
        r = dispatched(pc, tool.as_ref(), &cx, dispatch) => r,
    };
    match result {
        Ok(mut output) => {
            if pc.traits.result_limit == ResultLimit::Global {
                clip(&mut output);
            }
            let status = if output.is_error {
                ItemStatus::Failed
            } else {
                ItemStatus::Completed
            };
            finish(output, status)
        }
        Err(ToolError::Cancelled) => finish(
            ToolOutput::error(INTERRUPTED_MARKER),
            ItemStatus::Interrupted,
        ),
        Err(e) => finish(ToolOutput::error(e.to_string()), ItemStatus::Failed),
    }
}

pub async fn execute_one_chain(
    call: PendingCall,
    cancel: &CancellationToken,
    cx: ToolContext,
    dispatch: &Dispatch<'_>,
) -> ToolOutcome {
    if cancel.is_cancelled() {
        return interrupted(&call);
    }
    run_one(&call, cx, cancel, Some(dispatch)).await
}

async fn dispatched(
    pc: &PendingCall,
    tool: &dyn Tool,
    cx: &ToolContext,
    dispatch: Option<&Dispatch<'_>>,
) -> Result<ToolOutput, ToolError> {
    let Some(dispatch) = dispatch else {
        return tool.call(pc.call.input.clone(), cx).await;
    };
    let kernel = Next::new(|call: ToolCall| {
        Box::pin(async move {
            tool.call(call.input, cx)
                .await
                .map_err(|error| match error {
                    ToolError::Cancelled => KernelError::new(ErrorCode::Interrupted, "cancelled"),
                    error => KernelError::new(ErrorCode::Internal, error.to_string()),
                })
        })
    });
    crate::chain::run::<crate::chain::tool::Call>(
        dispatch.links,
        kernel,
        dispatch.cx,
        pc.call.clone(),
        dispatch.report.clone(),
    )
    .await
    .map_err(|error| {
        if error.code == ErrorCode::Interrupted {
            ToolError::Cancelled
        } else {
            ToolError::Failed(error.message)
        }
    })
}

fn clip(output: &mut ToolOutput) {
    let mut budget = MAX_RESULT_CHARS;
    for part in &mut output.parts {
        if let ContentPart::Text { text } = part {
            let n = text.chars().count();
            if n > budget {
                let keep: String = text.chars().take(budget).collect();
                *text = format!("{keep}\n[truncated: {} more characters]", n - budget);
                budget = 0;
            } else {
                budget -= n;
            }
        }
    }
}

#[cfg(test)]
mod tests;
