use super::*;

pub(crate) struct Check;
impl Point for Check {
    type Input = ToolCall;
    type Output = CheckResult;
    const POINT: LinkPoint = LinkPoint::ToolCheck;
    fn dispatch<'a>(
        link: &'a dyn Interceptor,
        cx: &'a HookContext,
        input: Self::Input,
        next: Next<'a, Self::Input, Self::Output>,
    ) -> BoxFuture<'a, Result<Self::Output, KernelError>> {
        Box::pin(link.tool_check(cx, input, next))
    }
    fn tool(input: &ToolCall) -> Option<&str> {
        Some(&input.name)
    }
    fn needs_baseline(tier: Tier, _: &CheckResult) -> bool {
        tier != Tier::Full
    }
    fn accepts(tier: Tier, before: Option<&CheckResult>, after: &CheckResult) -> bool {
        tier == Tier::Full
            || before.is_some_and(|before| {
                before.call == after.call && tightens(&before.decision, &after.decision)
            })
    }
}

pub(crate) struct Call;
impl Point for Call {
    type Input = ToolCall;
    type Output = ToolOutput;
    const POINT: LinkPoint = LinkPoint::ToolCall;
    fn dispatch<'a>(
        link: &'a dyn Interceptor,
        cx: &'a HookContext,
        input: Self::Input,
        next: Next<'a, Self::Input, Self::Output>,
    ) -> BoxFuture<'a, Result<Self::Output, KernelError>> {
        Box::pin(link.tool_call(cx, input, next))
    }
    fn tool(input: &ToolCall) -> Option<&str> {
        Some(&input.name)
    }
}

use crate::gate::{GateInput, hook_applies};
/// What the `BeforeTool` hooks decided; the first non-`Continue` wins.
enum HookVerdict {
    Continue,
    /// A hook wants a person to answer, for this reason.
    Ask(String),
    Refused(String),
}

/// Run the hooks that claim this call. They may rewrite its input.
async fn run_hooks(call: &mut ToolCall, hooks: &[Arc<dyn Hook>], cx: &HookContext) -> HookVerdict {
    let name = call.name.clone();
    let applicable: Vec<&Arc<dyn Hook>> = hooks
        .iter()
        .filter(|h| hook_applies(&h.matcher(), HookPoint::BeforeTool, Some(&name)))
        .collect();
    for hook in applicable {
        match hook.before_tool(call, cx).await {
            HookOutcome::Continue | HookOutcome::Redirect { .. } => {}
            HookOutcome::Deny { reason } | HookOutcome::Block { reason } => {
                return HookVerdict::Refused(format!("Denied by hook {}: {reason}", hook.id()));
            }
            HookOutcome::Ask { reason } => return HookVerdict::Ask(reason),
        }
    }
    HookVerdict::Continue
}

pub(crate) async fn check_kernel(
    mut call: ToolCall,
    input: &GateInput<'_>,
) -> Result<CheckResult, KernelError> {
    let forced = match run_hooks(&mut call, input.hooks, input.hook_cx).await {
        HookVerdict::Continue => None,
        HookVerdict::Ask(reason) => Some(reason),
        HookVerdict::Refused(message) => {
            return Ok(CheckResult {
                call,
                decision: Decision::Deny {
                    reason: Reason::Hook { hook: message },
                },
            });
        }
    };
    let Some(tool) = &input.tool else {
        return Ok(CheckResult {
            call,
            decision: Decision::Allow {
                reason: Reason::Default,
            },
        });
    };
    let traits = tool.traits(&call.input);
    let subjects = tool.subjects(&call.input, input.cwd);
    let confirm = tool.confirm(&call.input).or(forced);
    let policy = input
        .policy
        .decide(PolicyInput {
            call: &call,
            traits: &traits,
            subjects: &subjects,
            confirm: confirm.as_deref(),
            session: input.session,
            cwd: input.cwd,
        })
        .await;
    let decision = match (policy, confirm) {
        (deny @ Decision::Deny { .. }, _) => deny,
        (Decision::Ask { scope, .. }, Some(detail)) => Decision::Ask {
            scope,
            reason: Reason::Confirm { detail },
        },
        (Decision::Allow { .. }, Some(detail)) => Decision::Ask {
            scope: None,
            reason: Reason::Confirm { detail },
        },
        (decision, _) => decision,
    };
    Ok(CheckResult { call, decision })
}

pub(crate) async fn after(
    call: &ToolCall,
    output: &ToolOutput,
    hooks: &[Arc<dyn Hook>],
    cx: &HookContext,
) -> bool {
    let mut stop = false;
    for hook in hooks
        .iter()
        .filter(|hook| hook_applies(&hook.matcher(), HookPoint::AfterTool, Some(&call.name)))
    {
        if matches!(
            hook.after_tool(call, output, cx).await,
            HookOutcome::Block { .. }
        ) {
            stop = true;
        }
    }
    stop
}
