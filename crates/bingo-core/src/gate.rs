//! The permission gate: hooks, then the one policy, then a person when the
//! answer is `Ask`. The policy never resolves `Ask`; the gate does, so the
//! "unreachable" arm of the old design cannot exist.

use std::path::Path;
use std::sync::Arc;

use async_trait::async_trait;
use bingo_sdk::*;

use crate::executor::Gate;

mod asking;
pub use asking::{Answered, Asked, GatePrompter};

mod prompt;
use prompt::{Reply, ask_person};

/// With no policy registered: read-only trusted tools run, everything else asks.
#[derive(Debug, Default)]
pub struct DefaultPolicy;

#[async_trait]
impl PermissionPolicy for DefaultPolicy {
    fn id(&self) -> &str {
        "bingo.default"
    }

    async fn decide(&self, input: PolicyInput<'_>) -> Decision {
        if input.traits.trusted && input.traits.read_only && input.confirm.is_none() {
            Decision::Allow {
                reason: Reason::ReadOnly,
            }
        } else {
            Decision::Ask {
                reason: Reason::Default,
                scope: None,
            }
        }
    }
}

/// Whether a hook wants this point for this tool. `tool` is an exact name or a `prefix*`.
pub fn hook_applies(matcher: &HookMatcher, point: HookPoint, tool: Option<&str>) -> bool {
    if !matcher.points.is_empty() && !matcher.points.contains(&point) {
        return false;
    }
    match (&matcher.tool, tool) {
        (None, _) => true,
        (Some(_), None) => false,
        (Some(pattern), Some(name)) => match pattern.strip_suffix('*') {
            Some(prefix) => name.starts_with(prefix),
            None => pattern == name,
        },
    }
}

pub struct GateInput<'a> {
    pub session: &'a SessionId,
    pub cwd: &'a Path,
    pub item: &'a ItemId,
    pub call: ToolCall,
    pub tool: Option<Arc<dyn Tool>>,
    pub policy: &'a dyn PermissionPolicy,
    pub links: &'a [crate::chain::Link],
    pub hooks: &'a [Arc<dyn Hook>],
    pub hook_cx: &'a HookContext,
}

pub struct Gated {
    pub allowed_by: Option<Reason>,
    pub notices: Vec<ItemBody>,
    pub call: ToolCall,
    pub traits: ToolTraits,
    pub gate: Gate,
    /// The receipt to record when a person decided.
    pub receipt: Option<ItemBody>,
}

impl Gated {
    /// Through the gate with nobody asked, so nothing to receipt.
    fn allowed(call: ToolCall, traits: ToolTraits, reason: Reason) -> Self {
        Self {
            allowed_by: match reason {
                reason @ (Reason::Mode { .. } | Reason::Rule { .. } | Reason::Hook { .. }) => {
                    Some(reason)
                }
                _ => None,
            },
            notices: Vec::new(),
            call,
            traits,
            gate: Gate::Allowed,
            receipt: None,
        }
    }

    /// Stopped by a hook or the policy, so nobody was asked either.
    fn denied(call: ToolCall, traits: ToolTraits, message: String) -> Self {
        Self {
            allowed_by: None,
            notices: Vec::new(),
            call,
            traits,
            gate: Gate::Denied { message },
            receipt: None,
        }
    }
}

fn receipt_for(
    item: &ItemId,
    call: &ToolCall,
    decision: DecisionKind,
    feedback: Option<String>,
    asked: &Asked,
) -> ItemBody {
    ItemBody::PermissionReceipt {
        asked_at: Some(asked.opened_at),
        item: Some(item.clone()),
        interaction: asked.id.clone(),
        tool: call.name.clone(),
        decision,
        feedback,
    }
}

/// A decision made at the call's position in the round, before any ask.
pub(crate) enum Decided {
    Settled(Box<Gated>),
    Ask(Box<Waiting>),
}

pub(crate) struct Waiting {
    tool: Arc<dyn Tool>,
    call: ToolCall,
    traits: ToolTraits,
    scope: Option<String>,
    reason: Reason,
    blocked: Option<String>,
    notices: Vec<ItemBody>,
}

impl Decided {
    pub(crate) fn action(&self) -> crate::round::Decision {
        match self {
            Self::Settled(gated) => match gated.gate {
                Gate::Allowed => crate::round::Decision::Allowed {
                    safe: gated.traits.concurrency_safe,
                },
                Gate::Denied { .. } => crate::round::Decision::Denied,
            },
            Self::Ask(_) => crate::round::Decision::Ask,
        }
    }
}

/// The standalone gate uses the same decision and asking steps as a round.
pub async fn gate_call(input: GateInput<'_>, prompter: &dyn GatePrompter) -> Gated {
    let decided = decide(&input).await;
    ask(input, decided, prompter).await
}

pub(crate) async fn decide(input: &GateInput<'_>) -> Decided {
    check(input, false).await
}

async fn check(input: &GateInput<'_>, changed: bool) -> Decided {
    let report = crate::chain::Report::default();
    let hook_cx = HookContext {
        item: Some(input.item.clone()),
        ..input.hook_cx.clone()
    };
    let input = GateInput {
        session: input.session,
        cwd: input.cwd,
        item: input.item,
        call: input.call.clone(),
        tool: input.tool.clone(),
        policy: input.policy,
        links: input.links,
        hooks: input.hooks,
        hook_cx: &hook_cx,
    };
    let checked = crate::chain::run::<crate::chain::tool::Check>(
        input.links,
        Next::new(|call| Box::pin(crate::chain::tool::check_kernel(call, &input))),
        &hook_cx,
        input.call.clone(),
        report.clone(),
    )
    .await;
    let mut decided = match checked {
        Ok(checked) => resolve_check(checked, &input, changed),
        Err(error) => Decided::Settled(Box::new(Gated::denied(
            input.call.clone(),
            ToolTraits::default(),
            error.message,
        ))),
    };
    match &mut decided {
        Decided::Settled(gated) => gated.notices = report.take(),
        Decided::Ask(waiting) => waiting.notices = report.take(),
    }
    decided
}

pub(crate) async fn ask(
    mut input: GateInput<'_>,
    mut decided: Decided,
    prompter: &dyn GatePrompter,
) -> Gated {
    let mut changed = false;
    let mut asked = None;
    let mut notices = Vec::new();
    loop {
        let result = match decided {
            Decided::Settled(gated) => Ok(*gated),
            Decided::Ask(mut waiting) => {
                notices.append(&mut waiting.notices);
                ask_waiting(*waiting, &input, prompter, &mut asked).await
            }
        };
        match result {
            Ok(mut result) => {
                if changed
                    && result.gate == Gate::Allowed
                    && let Some(asked) = &asked
                {
                    result.receipt = Some(receipt_for(
                        input.item,
                        &result.call,
                        changed_decision(result.receipt.as_ref()),
                        None,
                        asked,
                    ));
                }
                notices.append(&mut result.notices);
                result.notices = notices;
                return result;
            }
            Err(call) => {
                input.call = call;
                changed = true;
                decided = check(&input, true).await;
            }
        }
    }
}

fn changed_decision(receipt: Option<&ItemBody>) -> DecisionKind {
    if matches!(
        receipt,
        Some(ItemBody::PermissionReceipt {
            decision: DecisionKind::AllowSession,
            ..
        })
    ) {
        DecisionKind::AllowChangedSession
    } else {
        DecisionKind::AllowChanged
    }
}

fn resolve_check(checked: CheckResult, input: &GateInput<'_>, changed: bool) -> Decided {
    let CheckResult { call, decision } = checked;
    let Some(tool) = input.tool.clone() else {
        let message = match decision {
            Decision::Deny {
                reason: Reason::Hook { hook },
            } => hook,
            _ => crate::executor::tool_not_found(&call.name),
        };
        return Decided::Settled(Box::new(Gated::denied(
            call,
            ToolTraits::default(),
            message,
        )));
    };
    let traits = tool.traits(&call.input);
    let (scope, reason, blocked) = match decision {
        Decision::Allow { reason } => {
            let mut result = Gated::allowed(call, traits, reason);
            if changed {
                result.allowed_by = None;
            }
            return Decided::Settled(Box::new(result));
        }
        Decision::Deny { reason } => {
            let message = match &reason {
                Reason::Hook { hook } => hook.clone(),
                reason => format!("Permission denied ({})", describe(reason)),
            };
            if !changed {
                return Decided::Settled(Box::new(Gated::denied(call, traits, message)));
            }
            (None, reason, Some(message))
        }
        Decision::Ask { scope, reason } => (scope, reason, None),
    };
    Decided::Ask(Box::new(Waiting {
        tool,
        call,
        traits,
        scope,
        reason,
        blocked,
        notices: Vec::new(),
    }))
}

async fn ask_waiting(
    waiting: Waiting,
    input: &GateInput<'_>,
    prompter: &dyn GatePrompter,
    asked: &mut Option<Asked>,
) -> Result<Gated, ToolCall> {
    let Waiting {
        tool,
        call,
        traits,
        scope,
        reason,
        blocked,
        ..
    } = waiting;
    let subjects = tool.subjects(&call.input, input.cwd);
    let confirm = match reason {
        Reason::Confirm { detail } => Some(detail),
        _ => tool.confirm(&call.input),
    };
    let policy_input = PolicyInput {
        call: &call,
        traits: &traits,
        subjects: &subjects,
        confirm: confirm.as_deref(),
        session: input.session,
        cwd: input.cwd,
    };
    let response = ask_person(
        policy_input,
        tool.as_ref(),
        scope,
        blocked,
        input.policy,
        prompter,
    )
    .await;
    resolve_reply(call, traits, input.item, response, asked)
}

fn resolve_reply(
    mut call: ToolCall,
    traits: ToolTraits,
    item: &ItemId,
    response: Result<(Reply, Asked), KernelError>,
    asked: &mut Option<Asked>,
) -> Result<Gated, ToolCall> {
    let Ok((reply, mut current)) = response else {
        return Ok(Gated::denied(
            call,
            traits,
            "Permission interaction closed".into(),
        ));
    };
    if let Some(first) = asked {
        current.opened_at = first.opened_at;
    }
    *asked = Some(current.clone());
    match reply {
        Reply::Changed(value) => {
            call.input = value;
            Err(call)
        }
        Reply::Decided(gate, decision, feedback) => {
            let receipt = Some(receipt_for(item, &call, decision, feedback, &current));
            Ok(Gated {
                allowed_by: None,
                call,
                traits,
                gate,
                receipt,
                notices: Vec::new(),
            })
        }
    }
}

fn describe(reason: &Reason) -> String {
    match reason {
        Reason::Rule { rule, .. } => format!("rule {rule}"),
        Reason::Mode { mode } => format!("mode {mode}"),
        Reason::Hook { hook } => format!("hook {hook}"),
        Reason::Safety { detail } => format!("safety: {detail}"),
        Reason::ReadOnly => "read-only".into(),
        Reason::Confirm { detail } => detail.clone(),
        Reason::Default => "default".into(),
    }
}

/// `Name {"k":"v"}` clipped to one line for the prompt.
/// What a person is asked to approve: the tool and what it touches — its
/// subjects when it names any (paths, a command, a url, a name), else its
/// input, so a tool with no subjects still shows something.
pub fn summarize(call: &ToolCall, subjects: &[Subject]) -> String {
    let target = if subjects.is_empty() {
        call.input.to_string()
    } else {
        subjects
            .iter()
            .map(subject_text)
            .collect::<Vec<_>>()
            .join(" ")
    };
    let target: String = target
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .chars()
        .take(120)
        .collect();
    format!("{} {target}", call.name)
}

fn subject_text(subject: &Subject) -> String {
    match subject {
        Subject::Path { path } => path.display().to_string(),
        Subject::Command { command } => command.clone(),
        Subject::Url { url } => url.clone(),
        Subject::Name { name } => name.clone(),
    }
}

#[cfg(test)]
mod summary_tests {
    use super::*;
    use serde_json::json;

    fn call(name: &str, input: serde_json::Value) -> ToolCall {
        ToolCall {
            call_id: "c".into(),
            name: name.into(),
            input,
        }
    }

    #[test]
    fn a_summary_names_the_subjects_and_falls_back_to_the_input() {
        let write = call("Write", json!({"file_path": "note.txt", "content": "x\n"}));
        let path = Subject::Path {
            path: "/work/note.txt".into(),
        };
        assert_eq!(summarize(&write, &[path]), "Write /work/note.txt");
        let bash = call("Bash", json!({"command": "ls\n  -la"}));
        let command = Subject::Command {
            command: "ls\n  -la".into(),
        };
        assert_eq!(summarize(&bash, &[command]), "Bash ls -la");
        assert_eq!(
            summarize(&call("Echo", json!({"v": 1})), &[]),
            "Echo {\"v\":1}"
        );
    }
}

#[cfg(test)]
#[path = "chain/hook_contract_gate.rs"]
mod hook_contract;

#[cfg(test)]
mod tests;
