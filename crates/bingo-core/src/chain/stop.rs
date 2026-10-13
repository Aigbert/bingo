use super::*;

pub(crate) struct Stop;
impl Point for Stop {
    type Input = ();
    type Output = StopResult;
    const POINT: LinkPoint = LinkPoint::TurnStop;
    fn dispatch<'a>(
        link: &'a dyn Interceptor,
        cx: &'a HookContext,
        input: Self::Input,
        next: Next<'a, Self::Input, Self::Output>,
    ) -> BoxFuture<'a, Result<Self::Output, KernelError>> {
        Box::pin(link.turn_stop(cx, input, next))
    }
}
