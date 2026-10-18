use super::*;

pub(crate) struct Check;
impl Point for Check {
    type Input = ToolCall;
    type Output = CheckResult;
    const POINT: LinkPoint = bingo_sdk::catalogue::TOOL_CHECK;
    fn dispatch<'a>(
        link: &'a dyn Interceptor,
        cx: &'a HookContext,
        input: Self::Input,
        next: Next<'a, Self::Input, Self::Output>,
    ) -> BoxFuture<'a, Result<Self::Output, KernelError>> {
        Box::pin(link.tool_check(cx, input, next))
    }
    fn pinned(before: &ToolCall, after: &ToolCall) -> bool {
        before.name == after.name && before.call_id == after.call_id
    }
    fn tool(input: &ToolCall) -> Option<&str> {
        Some(&input.name)
    }
    fn needs_baseline(tier: Tier, output: &CheckResult) -> bool {
        tier != Tier::Full && !matches!(output.decision, Decision::Deny { .. })
    }
    fn accepts(
        _entry: &bingo_sdk::catalogue::Entry,
        tier: Tier,
        before: Option<&CheckResult>,
        after: &CheckResult,
    ) -> bool {
        tier == Tier::Full
            || (before.is_none() && matches!(after.decision, Decision::Deny { .. }))
            || before.is_some_and(|before| {
                before.call == after.call && tightens(&before.decision, &after.decision)
            })
    }
}

pub(crate) struct Call;
impl Point for Call {
    type Input = ToolCall;
    type Output = ToolOutput;
    const POINT: LinkPoint = bingo_sdk::catalogue::TOOL_CALL;
    fn dispatch<'a>(
        link: &'a dyn Interceptor,
        cx: &'a HookContext,
        input: Self::Input,
        next: Next<'a, Self::Input, Self::Output>,
    ) -> BoxFuture<'a, Result<Self::Output, KernelError>> {
        Box::pin(link.tool_call(cx, input, next))
    }
    fn pinned(before: &ToolCall, after: &ToolCall) -> bool {
        before.name == after.name && before.call_id == after.call_id
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

pub(crate) async fn after_outcomes(
    outcomes: &[ToolOutcome],
    transcript: &Transcript,
    hooks: &crate::turn::HookSet,
    cx: &HookContext,
) -> bool {
    let hooks = hooks.gather().await;
    let mut stop = false;
    for outcome in outcomes {
        let Some(item) = transcript.get(&outcome.item) else {
            continue;
        };
        let ItemBody::ToolCall {
            call_id,
            name,
            input,
            ..
        } = &item.body
        else {
            continue;
        };
        let call = ToolCall {
            call_id: call_id.clone(),
            name: name.clone(),
            input: input.clone(),
        };
        let cx = HookContext {
            item: Some(outcome.item.clone()),
            ..cx.clone()
        };
        stop |= after(&call, &outcome.output, &hooks, &cx).await;
    }
    stop
}

pub(crate) struct AskVia<'a> {
    pub host: &'a dyn crate::turn::TurnHost,
    pub item: ItemId,
}
#[async_trait::async_trait]
impl crate::gate::GatePrompter for AskVia<'_> {
    async fn ask(
        &self,
        kind: InteractionKind,
        answers: Vec<AnswerSpec>,
    ) -> Result<crate::gate::Answered, KernelError> {
        self.host.ask(Some(self.item.clone()), kind, answers).await
    }
}

#[cfg(test)]
mod item_tests {
    use super::*;

    struct SeesItem;
    #[async_trait::async_trait]
    impl Hook for SeesItem {
        fn id(&self) -> &str {
            "item-observer"
        }
        fn matcher(&self) -> HookMatcher {
            HookMatcher::default()
        }
        async fn after_tool(
            &self,
            call: &ToolCall,
            _: &ToolOutput,
            cx: &HookContext,
        ) -> HookOutcome {
            assert_eq!(
                cx.item.as_ref().map(ItemId::as_str),
                Some(call.call_id.as_str())
            );
            HookOutcome::Continue
        }
    }

    fn outcome(id: &str, transcript: &mut Transcript) -> ToolOutcome {
        let Event::ItemCompleted { mut item } = serde_json::from_str(include_str!(
            "../../../bingo-sdk/fixtures/record/event/itemCompleted.json"
        ))
        .unwrap() else {
            panic!("item")
        };
        item.id = ItemId::from_raw(id);
        item.body = ItemBody::ToolCall {
            call_id: id.into(),
            name: "Example".into(),
            input: serde_json::json!({}),
            output: None,
            progress: None,
            duration_ms: None,
            allowed_by: None,
            asked_at: None,
            began_at: None,
            quiet: false,
        };
        transcript.apply(&Event::ItemCompleted { item });
        ToolOutcome {
            item: ItemId::from_raw(id),
            call_id: id.into(),
            output: ToolOutput::text("done"),
            status: ItemStatus::Completed,
            duration_ms: 0,
        }
    }

    #[tokio::test]
    async fn after_hooks_receive_each_outcomes_own_item() {
        let mut transcript = Transcript::new();
        let outcomes = [
            outcome("itm_first", &mut transcript),
            outcome("itm_second", &mut transcript),
        ];
        let hooks = crate::turn::HookSet::fixed(vec![Arc::new(SeesItem)]);
        let cx = HookContext {
            item: None,
            session: SessionId::from_raw("session"),
            turn: None,
            cwd: "/".into(),
            provider: None,
            model: None,
            host: bingo_sdk::testing::NoHost::handle(),
        };
        assert!(!after_outcomes(&outcomes, &transcript, &hooks, &cx).await);
        assert_eq!(cx.item, None);
    }
}
