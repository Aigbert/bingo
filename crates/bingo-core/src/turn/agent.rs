//! One agent run, sharing the model stream's accumulator and journal.
use super::*;
use crate::accumulator::Accumulator;
use futures::StreamExt;
use std::collections::HashMap;
use tokio::sync::mpsc;

impl Turn<'_> {
    pub(super) async fn agent_turn(&mut self, agent: Arc<dyn Agent>) -> TurnStatus {
        let (events, mut judged) = mpsc::unbounded_channel();
        let cancel = self.cancel.child_token();
        let _stop = cancel.clone().drop_guard();
        let request = self.agent_request(agent.id(), cancel, events);
        let mut stream = match self.racing(agent.run(request)).await {
            Some(Ok(stream)) => stream,
            Some(Err(error)) => return provider_failed(error),
            None => return self.interrupted_status(),
        };
        let mut acc = Accumulator::new(self.id.clone(), 0);
        let mut calls = HashMap::new();
        self.host.streaming(true);
        let status = self
            .read_agent(&mut stream, &mut judged, &mut acc, &mut calls)
            .await;
        self.host.streaming(false);
        let (emits, finished) = acc.finish(!matches!(status, TurnStatus::Completed));
        for emit in emits {
            self.publish(emit);
        }
        if let Some((_, window)) = finished.context {
            self.model
                .learned
                .reported(agent.id(), &self.model.id, window);
        }
        self.account(
            &finished,
            ContextUsage {
                parts: None,
                used: 0,
                window: self.model.capabilities.context_window,
                trigger: self.model.capabilities.context_window,
            },
        );
        while let Ok(event) = judged.try_recv() {
            emit_and_fold(&mut self.transcript, self.host, event);
        }
        self.close_external(calls, &status);
        if matches!(status, TurnStatus::Completed) {
            if let Some(error) = finished.error {
                return provider_failed(error);
            }
            if finished.finish_reason.is_none() {
                return failed("the agent stream ended without a finish");
            }
        }
        status
    }

    fn agent_request(
        &self,
        name: &str,
        cancel: CancellationToken,
        events: mpsc::UnboundedSender<Event>,
    ) -> AgentTurn {
        let judge = Arc::new(crate::judge::TurnJudge {
            name: name.into(),
            cwd: self.cfg.cwd.clone(),
            policy: self.cfg.policy.clone(),
            person: self.cfg.call_host.gate.clone(),
            hooks: self.hooks.clone(),
            cx: self.hook_cx.clone(),
            cancel,
            events,
        });
        let input = self
            .transcript
            .items()
            .iter()
            .rev()
            .find_map(|item| match &item.body {
                ItemBody::User { parts, origin } if !origin.is_context() => Some(parts.clone()),
                _ => None,
            })
            .unwrap_or_default();
        AgentTurn {
            session: self.cfg.session.id.clone(),
            cwd: self.cfg.cwd.clone(),
            input,
            tools: self.tool_specs(),
            model: self.model.id.clone(),
            reasoning: self.model.reasoning,
            judge,
        }
    }

    async fn read_agent(
        &mut self,
        stream: &mut AgentStream,
        judged: &mut mpsc::UnboundedReceiver<Event>,
        acc: &mut Accumulator,
        calls: &mut HashMap<String, Item>,
    ) -> TurnStatus {
        loop {
            tokio::select! {
                biased;
                _ = self.cancel.cancelled() => return self.interrupted_status(),
                Some(event) = judged.recv() => emit_and_fold(&mut self.transcript, self.host, event),
                event = stream.next() => match event {
                    Some(Ok(event)) => {
                        let cancel = self.cancel.clone();
                        tokio::select! {
                            biased;
                            _ = cancel.cancelled() => return self.interrupted_status(),
                            result = self.agent_event(event, acc, calls) => if let Err(message) = result { return failed(message); },
                        }
                    },
                    Some(Err(error)) => return provider_failed(error),
                    None => return TurnStatus::Completed,
                }
            }
        }
    }

    async fn agent_event(
        &mut self,
        event: AgentEvent,
        acc: &mut Accumulator,
        calls: &mut HashMap<String, Item>,
    ) -> Result<(), String> {
        match event {
            AgentEvent::Said(event) => {
                if matches!(
                    event,
                    ModelEvent::ToolCall { .. }
                        | ModelEvent::ToolInputStart { .. }
                        | ModelEvent::ToolInputDelta { .. }
                        | ModelEvent::ToolInputEnd { .. }
                ) {
                    return Err("an agent sent a model tool request; external calls must use AgentEvent::Call".into());
                }
                for emit in acc.push(event) {
                    self.publish(emit);
                }
            }
            AgentEvent::Call(call) | AgentEvent::Update(call) => {
                self.external(call, None, calls);
            }
            AgentEvent::Done { call, output } => {
                let item = self.external(call.clone(), None, calls);
                let hooks = self.hooks.gather().await;
                let links: Vec<_> = self
                    .hooks
                    .links()
                    .await
                    .into_iter()
                    .map(|mut link| {
                        link.tier = Tier::Tighten;
                        link
                    })
                    .collect();
                let report = crate::chain::Report::default();
                let cx = HookContext {
                    item: Some(item),
                    ..self.hook_cx.clone()
                };
                let tool = call.tool_call(self.model.endpoint.id());
                let kernel = Next::new(|_| Box::pin(async { Ok(output.clone()) }));
                crate::chain::run::<crate::chain::tool::Call>(
                    &links,
                    kernel,
                    &cx,
                    tool.clone(),
                    report.clone(),
                )
                .await
                .map_err(|e| e.to_string())?;
                for body in report.take() {
                    self.record(body);
                }
                let blocked = crate::chain::tool::after(&tool, &output, &hooks, &cx).await;
                self.external(call, Some(output), calls);
                if blocked {
                    return Err("an AfterTool hook blocked the agent turn".into());
                }
            }
        }
        Ok(())
    }

    fn external(
        &mut self,
        call: ExternalCall,
        output: Option<ToolOutput>,
        calls: &mut HashMap<String, Item>,
    ) -> ItemId {
        let tool = call.tool_call(self.model.endpoint.id());
        let existing = calls.remove(&call.id);
        let started = existing.is_none();
        let body = ItemBody::ToolCall {
            call_id: tool.call_id,
            name: tool.name,
            input: tool.input,
            output: output.clone(),
            progress: None,
            duration_ms: None,
            allowed_by: None,
            quiet: false,
        };
        let mut item =
            existing.unwrap_or_else(|| mint(&self.id, 0, body.clone(), ItemStatus::Running));
        item.mark_external();
        item.body = body;
        if started {
            emit_and_fold(
                &mut self.transcript,
                self.host,
                Event::ItemStarted { item: item.clone() },
            );
        }
        let id = item.id.clone();
        if let Some(output) = output {
            item.status = if output.is_error {
                ItemStatus::Failed
            } else {
                ItemStatus::Completed
            };
            item.completed_at = Some(Timestamp::now());
            emit_and_fold(
                &mut self.transcript,
                self.host,
                Event::ItemCompleted { item },
            );
        } else {
            if !started {
                emit_and_fold(
                    &mut self.transcript,
                    self.host,
                    Event::ItemUpdated { item: item.clone() },
                );
            }
            calls.insert(call.id, item);
        }
        id
    }

    fn close_external(&mut self, calls: HashMap<String, Item>, status: &TurnStatus) {
        for (_, mut item) in calls {
            item.status = if matches!(status, TurnStatus::Interrupted { .. }) {
                ItemStatus::Interrupted
            } else {
                ItemStatus::Failed
            };
            item.completed_at = Some(Timestamp::now());
            emit_and_fold(
                &mut self.transcript,
                self.host,
                Event::ItemCompleted { item },
            );
        }
    }
}

fn failed(message: impl Into<String>) -> TurnStatus {
    TurnStatus::Failed {
        error: KernelError::new(ErrorCode::ProviderUnavailable, message),
    }
}

fn provider_failed(error: ProviderError) -> TurnStatus {
    let code = match error {
        ProviderError::ContextOverflow { .. } => ErrorCode::ContextOverflow,
        _ => ErrorCode::ProviderUnavailable,
    };
    TurnStatus::Failed {
        error: KernelError::new(code, error.to_string()),
    }
}
