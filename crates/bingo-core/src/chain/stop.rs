use super::*;

pub(crate) struct Stop;
impl Point for Stop {
    type Input = ();
    type Output = StopResult;
    const POINT: LinkPoint = bingo_sdk::catalogue::TURN_STOP;
    fn dispatch<'a>(
        link: &'a dyn Interceptor,
        cx: &'a HookContext,
        input: Self::Input,
        next: Next<'a, Self::Input, Self::Output>,
    ) -> BoxFuture<'a, Result<Self::Output, KernelError>> {
        Box::pin(link.turn_stop(cx, input, next))
    }
}

pub(crate) async fn kernel(
    hooks: &[Arc<dyn Hook>],
    cx: &HookContext,
) -> Result<StopResult, KernelError> {
    for hook in hooks {
        if let HookOutcome::Block { reason } = hook.on_stop(cx).await {
            return Ok(StopResult::Continue {
                origin: format!("hook:{}", hook.id()),
                reason,
            });
        }
    }
    Ok(StopResult::Complete)
}

pub(crate) async fn dispatch(
    hooks: &crate::turn::HookSet,
    cx: &HookContext,
    report: Report,
) -> Result<StopResult, KernelError> {
    let matched = hooks.at(HookPoint::Stop, None).await;
    run::<Stop>(
        &hooks.links().await,
        Next::new(|()| Box::pin(kernel(&matched, cx))),
        cx,
        (),
        report,
    )
    .await
}
