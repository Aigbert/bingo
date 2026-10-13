use super::*;

pub(crate) struct Compose;
impl Point for Compose {
    type Input = ModelRequest;
    type Output = ModelRequest;
    const POINT: LinkPoint = LinkPoint::PromptCompose;
    fn dispatch<'a>(
        link: &'a dyn Interceptor,
        cx: &'a HookContext,
        input: Self::Input,
        next: Next<'a, Self::Input, Self::Output>,
    ) -> BoxFuture<'a, Result<Self::Output, KernelError>> {
        Box::pin(link.prompt_compose(cx, input, next))
    }
}
