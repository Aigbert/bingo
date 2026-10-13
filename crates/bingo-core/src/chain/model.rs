use super::*;

pub(crate) struct Model;
impl Point for Model {
    type Input = ModelRequest;
    type Output = Result<StepStream, ProviderError>;
    const POINT: LinkPoint = LinkPoint::TurnStep;
    fn dispatch<'a>(
        link: &'a dyn Interceptor,
        cx: &'a HookContext,
        input: Self::Input,
        next: Next<'a, Self::Input, Self::Output>,
    ) -> BoxFuture<'a, Result<Self::Output, KernelError>> {
        Box::pin(link.turn_step(cx, input, next))
    }
}
