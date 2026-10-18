//! A round walks once; each decision belongs to the call it has reached.

use super::*;
use crate::executor::{Dispatch, InFlight};
use crate::gate::{self, Decided, Gated};
use crate::round::{Action, next};

impl Turn<'_> {
    pub(super) async fn run_tools(&mut self, finished: &Finished) -> Vec<ToolOutcome> {
        let mut outcomes = Vec::with_capacity(finished.tool_calls.len());
        let report = crate::chain::Report::default();
        let links = self.hooks.links.clone();
        let cx = self.hook_cx.clone();
        let dispatch = Dispatch {
            links: &links,
            cx: &cx,
            report: report.clone(),
        };
        let mut flight = InFlight::default();
        if let Some(hooks) = self.racing(self.hooks.gather()).await {
            self.walk_calls(finished, &hooks, &dispatch, &mut flight, &mut outcomes)
                .await;
        }
        if self.cancel.is_cancelled() {
            for outcome in flight.cancelled() {
                self.land(outcome, &mut outcomes);
            }
        }
        self.interrupt_remaining(finished, &mut outcomes);
        for notice in report.take() {
            self.record(notice);
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
        &mut self,
        finished: &Finished,
        hooks: &[Arc<dyn Hook>],
        dispatch: &Dispatch<'a>,
        flight: &mut InFlight<'a>,
        outcomes: &mut Vec<ToolOutcome>,
    ) -> Option<()> {
        for (item, call) in &finished.tool_calls {
            while !flight.state().all_safe {
                self.land_next(flight, outcomes).await?;
            }
            let cfg = self.cfg;
            let tool = self
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
            let decided = self
                .progressing(flight, outcomes, gate::decide(&input))
                .await?;
            let gated = self
                .settle_reached(input, decided, flight, outcomes)
                .await?;
            let call = self.admit(item, gated, tool).await?;
            let cx = self.tool_context(&call);
            if let Some(outcome) =
                flight.start(call, cx, self.cancel.clone(), Some(dispatch.clone()))
            {
                self.land(outcome, outcomes);
            }
        }
        while next(flight.state(), None) == Action::Wait {
            self.land_next(flight, outcomes).await?;
        }
        Some(())
    }

    async fn admit(
        &mut self,
        item: &ItemId,
        mut gated: Gated,
        tool: Option<Arc<dyn Tool>>,
    ) -> Option<PendingCall> {
        for notice in gated.notices.drain(..) {
            self.record(notice);
        }
        if !self.record_receipt(gated.receipt.take()).await || self.cancel.is_cancelled() {
            return None;
        }
        self.mark_gated(item, &mut gated);
        Some(PendingCall {
            item: item.clone(),
            call: gated.call,
            tool,
            traits: gated.traits,
            gate: gated.gate,
        })
    }

    async fn settle_reached(
        &mut self,
        input: GateInput<'_>,
        decided: Decided,
        flight: &mut InFlight<'_>,
        outcomes: &mut Vec<ToolOutcome>,
    ) -> Option<Gated> {
        while next(flight.state(), Some(decided.action())) == Action::Wait {
            self.land_next(flight, outcomes).await?;
        }
        match decided {
            Decided::Settled(gated) => Some(*gated),
            decided @ Decided::Ask(_) => {
                let prompter = AskVia {
                    host: self.host,
                    item: input.item.clone(),
                };
                self.racing(gate::ask(input, decided, &prompter)).await
            }
        }
    }

    /// Earlier safe calls keep progressing while a reached call is deciding.
    async fn progressing<T>(
        &mut self,
        flight: &mut InFlight<'_>,
        outcomes: &mut Vec<ToolOutcome>,
        work: impl Future<Output = T>,
    ) -> Option<T> {
        tokio::pin!(work);
        let cancel = self.cancel.clone();
        racing(&cancel, async {
            loop {
                tokio::select! {
                    biased;
                    Some(outcome) = flight.next(), if flight.state().count > 0 => {
                        self.land(outcome, outcomes);
                    }
                    result = &mut work => return result,
                }
            }
        })
        .await
    }

    async fn land_next(
        &mut self,
        flight: &mut InFlight<'_>,
        outcomes: &mut Vec<ToolOutcome>,
    ) -> Option<()> {
        let outcome = self.racing(flight.next()).await??;
        self.land(outcome, outcomes);
        Some(())
    }

    fn tool_context(&self, call: &PendingCall) -> ToolContext {
        ToolContext {
            call_id: call.call.call_id.clone(),
            session: self.cfg.session.id.clone(),
            turn: self.id.clone(),
            item: call.item.clone(),
            cwd: self.cfg.cwd.clone(),
            cancel: self.cancel.child_token(),
            env: self.cfg.env.clone(),
            host: self.cfg.host.clone(),
            call: self.cfg.call_host.tool.clone(),
        }
    }

    fn interrupt_remaining(&mut self, finished: &Finished, outcomes: &mut Vec<ToolOutcome>) {
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
            self.land(executor::interrupted(&pending), outcomes);
        }
    }

    fn land(&mut self, outcome: ToolOutcome, outcomes: &mut Vec<ToolOutcome>) {
        if let Some(mut item) = self.transcript.get(&outcome.item).cloned() {
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
                &mut self.transcript,
                self.host,
                Event::ItemCompleted { item },
            );
        }
        outcomes.push(outcome);
    }

    fn mark_gated(&mut self, item_id: &ItemId, gated: &mut crate::gate::Gated) {
        if let Some(mut item) = self.transcript.get(item_id).cloned()
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
            emit_and_fold(&mut self.transcript, self.host, Event::ItemUpdated { item });
        }
    }
}
