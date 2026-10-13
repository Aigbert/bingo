//! The permission gate: hooks, then the one policy, then a person when the
//! answer is `Ask`. The policy never resolves `Ask`; the gate does, so the
//! "unreachable" arm of the old design cannot exist.

use std::path::Path;
use std::sync::Arc;

use async_trait::async_trait;
use bingo_sdk::*;

use crate::executor::Gate;

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
    pub notices: Vec<ItemBody>,
    pub call: ToolCall,
    pub traits: ToolTraits,
    pub gate: Gate,
    /// The receipt to record when a person decided.
    pub receipt: Option<ItemBody>,
}

impl Gated {
    /// Through the gate with nobody asked, so nothing to receipt.
    fn allowed(call: ToolCall, traits: ToolTraits) -> Self {
        Self {
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
            notices: Vec::new(),
            call,
            traits,
            gate: Gate::Denied { message },
            receipt: None,
        }
    }
}

/// Put the call to a person; their answer is the gate and, session-scoped, a rule.
async fn ask_person(
    input: PolicyInput<'_>,
    tool: &dyn Tool,
    scope: Option<String>,
    policy: &dyn PermissionPolicy,
    prompter: &dyn Prompter,
) -> (Gate, DecisionKind, Option<String>) {
    let kind = InteractionKind::Permission {
        tool: input.call.name.clone(),
        summary: input
            .confirm
            .map(str::to_string)
            .unwrap_or_else(|| summarize(input.call, input.subjects)),
        preview: tool.preview(&input.call.input, input.cwd),
        session_scope: scope.clone(),
    };
    let mut answers = vec![AnswerSpec::AllowOnce];
    if scope.is_some() {
        answers.push(AnswerSpec::AllowSession);
    }
    answers.push(AnswerSpec::Deny);
    match prompter.ask(kind, answers).await.unwrap_or(Answer::Cancel) {
        Answer::AllowOnce | Answer::Confirm => (Gate::Allowed, DecisionKind::Allow, None),
        Answer::AllowSession { scope } => {
            policy
                .on_verdict(input, &Verdict::Allow { scope: Some(scope) })
                .await;
            (Gate::Allowed, DecisionKind::AllowSession, None)
        }
        Answer::Deny { feedback } => {
            policy
                .on_verdict(
                    input,
                    &Verdict::Deny {
                        feedback: feedback.clone(),
                    },
                )
                .await;
            let gate = denied_by_user(feedback.as_deref());
            (gate, DecisionKind::Deny, feedback)
        }
        Answer::Cancel | Answer::Choice { .. } | Answer::Text { .. } | Answer::Form { .. } => (
            Gate::Denied {
                message: "Permission request cancelled".into(),
            },
            DecisionKind::Deny,
            None,
        ),
    }
}

fn denied_by_user(feedback: Option<&str>) -> Gate {
    let mut message = "Permission denied by the user".to_string();
    if let Some(f) = feedback {
        message.push_str(": ");
        message.push_str(f);
    }
    Gate::Denied { message }
}

fn receipt_for(call: &ToolCall, decision: DecisionKind, feedback: Option<String>) -> ItemBody {
    ItemBody::PermissionReceipt {
        interaction: InteractionId::from_raw("pending"),
        tool: call.name.clone(),
        decision,
        feedback,
    }
}

/// Decide one call: hooks, then the policy, then a person when the answer is `Ask`.
pub async fn gate_call(input: GateInput<'_>, prompter: &dyn Prompter) -> Gated {
    let report = crate::chain::Report::default();
    let checked = crate::chain::run::<crate::chain::tool::Check>(
        input.links,
        Next::new(|call| Box::pin(crate::chain::tool::check_kernel(call, &input))),
        input.hook_cx,
        input.call.clone(),
        report.clone(),
    )
    .await;
    let mut result = match checked {
        Ok(checked) => resolve_check(checked, &input, prompter).await,
        Err(error) => Gated::denied(input.call.clone(), ToolTraits::default(), error.message),
    };
    result.notices = report.take();
    result
}

async fn resolve_check(
    checked: CheckResult,
    input: &GateInput<'_>,
    prompter: &dyn Prompter,
) -> Gated {
    let CheckResult { call, decision } = checked;
    let traits = input
        .tool
        .as_ref()
        .map(|tool| tool.traits(&call.input))
        .unwrap_or_default();
    let (scope, reason) = match decision {
        Decision::Allow { .. } => return Gated::allowed(call, traits),
        Decision::Deny { reason } => {
            let message = match reason {
                Reason::Hook { hook } => hook,
                reason => format!("Permission denied ({})", describe(&reason)),
            };
            return Gated::denied(call, traits, message);
        }
        Decision::Ask { scope, reason } => (scope, reason),
    };
    let Some(tool) = &input.tool else {
        return Gated::denied(call, traits, "tool not found".into());
    };
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
    let (gate, decision, feedback) =
        ask_person(policy_input, tool.as_ref(), scope, input.policy, prompter).await;
    let receipt = receipt_for(&call, decision, feedback);
    Gated {
        call,
        traits,
        gate,
        receipt: Some(receipt),
        notices: Vec::new(),
    }
}

fn describe(reason: &Reason) -> String {
    match reason {
        Reason::Rule { rule } => format!("rule {rule}"),
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
