use super::*;

pub(crate) struct Submit;
impl Point for Submit {
    type Input = Input;
    type Output = SubmitResult;
    const POINT: LinkPoint = LinkPoint::PromptSubmit;
    fn dispatch<'a>(
        link: &'a dyn Interceptor,
        cx: &'a HookContext,
        input: Self::Input,
        next: Next<'a, Self::Input, Self::Output>,
    ) -> BoxFuture<'a, Result<Self::Output, KernelError>> {
        Box::pin(link.prompt_submit(cx, input, next))
    }
}

pub(crate) async fn kernel(
    mut input: Input,
    hooks: &[Arc<dyn Hook>],
    cx: &HookContext,
) -> Result<SubmitResult, KernelError> {
    for hook in hooks {
        match hook.on_submit(&mut input, cx).await {
            HookOutcome::Continue | HookOutcome::Ask { .. } => {}
            HookOutcome::Deny { reason } | HookOutcome::Block { reason } => {
                return Ok(SubmitResult::Rejected(KernelError::new(
                    ErrorCode::PermissionDenied,
                    reason,
                )));
            }
            HookOutcome::Redirect { session } => {
                return Ok(SubmitResult::Redirect { session, input });
            }
        }
    }
    Ok(SubmitResult::Accepted(input))
}
