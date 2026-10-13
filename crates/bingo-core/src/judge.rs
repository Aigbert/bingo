//! External calls use the same gate, with traits supplied by a closed table.
use crate::{
    executor::Gate,
    gate::{GateInput, gate_call},
    turn::HookSet,
};
use async_trait::async_trait;
use bingo_sdk::*;
use serde_json::Value;
use std::{
    path::{Path, PathBuf},
    sync::Arc,
};
use tokio::sync::mpsc;

pub fn traits_of(kind: CallKind) -> ToolTraits {
    ToolTraits {
        read_only: matches!(kind, CallKind::Read | CallKind::Search),
        edit: kind.writes(),
        destructive: kind == CallKind::Delete,
        ..ToolTraits::default()
    }
}

struct StandIn {
    call: ExternalCall,
    name: String,
}
#[async_trait]
impl Tool for StandIn {
    fn spec(&self) -> ToolSpec {
        ToolSpec {
            name: self.name.clone(),
            description: self.call.title.clone(),
            input_schema: serde_json::json!({"type":"object"}),
            meta: Default::default(),
        }
    }
    fn traits(&self, _: &Value) -> ToolTraits {
        traits_of(self.call.kind)
    }
    fn subjects(&self, _: &Value, cwd: &Path) -> Vec<Subject> {
        let mut subjects: Vec<_> = self
            .call
            .locations
            .iter()
            .map(|l| Subject::Path {
                path: if l.path.is_absolute() {
                    l.path.clone()
                } else {
                    cwd.join(&l.path)
                },
            })
            .collect();
        if self.call.kind == CallKind::Execute
            && let Some(command) = self.call.raw.get("command").and_then(Value::as_str)
        {
            subjects.push(Subject::Command {
                command: command.into(),
            });
        }
        subjects
    }
    async fn call(&self, _: Value, _: &ToolContext) -> Result<ToolOutput, ToolError> {
        Err(ToolError::Failed(
            "an external call cannot be executed here".into(),
        ))
    }
}

pub(crate) struct TurnJudge {
    pub name: String,
    pub cwd: PathBuf,
    pub policy: Arc<dyn PermissionPolicy>,
    pub hooks: HookSet,
    pub cx: HookContext,
    pub cancel: CancellationToken,
    pub events: mpsc::UnboundedSender<Event>,
}

struct Person<'a>(&'a HookContext);
#[async_trait]
impl Prompter for Person<'_> {
    async fn ask(
        &self,
        kind: InteractionKind,
        answers: Vec<AnswerSpec>,
    ) -> Result<Answer, KernelError> {
        self.0.host.ask(&self.0.session, kind, answers).await
    }
}

impl TurnJudge {
    async fn weigh(&self, external: ExternalCall) -> Verdict {
        let call = external.tool_call(&self.name);
        let stand_in = Arc::new(StandIn {
            call: external,
            name: call.name.clone(),
        });
        let item = ItemId::mint();
        let cx = HookContext {
            item: Some(item.clone()),
            ..self.cx.clone()
        };
        let hooks = self.hooks.gather().await;
        let links = self.hooks.links().await;
        let gated = gate_call(
            GateInput {
                session: &cx.session,
                cwd: &self.cwd,
                item: &item,
                call,
                tool: Some(stand_in),
                policy: self.policy.as_ref(),
                links: &links,
                hooks: &hooks,
                hook_cx: &cx,
            },
            &Person(&cx),
        )
        .await;
        for body in gated.notices.into_iter().chain(gated.receipt) {
            let item = super::turn::mint(
                cx.turn.as_ref().unwrap_or(&TurnId::from_raw("external")),
                0,
                body,
                ItemStatus::Completed,
            );
            let _ = self.events.send(Event::ItemCompleted { item });
        }
        match gated.gate {
            Gate::Allowed => Verdict::Allow { scope: None },
            Gate::Denied { message } => Verdict::Deny {
                feedback: Some(message),
            },
        }
    }
}

#[async_trait]
impl Judge for TurnJudge {
    async fn judge(&self, call: ExternalCall) -> Verdict {
        tokio::select! {
            biased;
            _ = self.cancel.cancelled() => Verdict::Deny { feedback: None },
            verdict = self.weigh(call) => verdict,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn kind_traits_fail_closed() {
        for kind in [
            CallKind::Read,
            CallKind::Search,
            CallKind::Edit,
            CallKind::Delete,
            CallKind::Move,
            CallKind::Execute,
            CallKind::Think,
            CallKind::Fetch,
            CallKind::Other,
        ] {
            let traits = traits_of(kind);
            assert!(!traits.trusted && !traits.concurrency_safe);
            assert_eq!(
                traits.read_only,
                matches!(kind, CallKind::Read | CallKind::Search)
            );
            assert_eq!(
                traits.edit,
                matches!(kind, CallKind::Edit | CallKind::Delete | CallKind::Move)
            );
            assert_eq!(traits.destructive, kind == CallKind::Delete);
        }
        assert_eq!(traits_of(CallKind::Other), ToolTraits::default());
    }
}
