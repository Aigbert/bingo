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
