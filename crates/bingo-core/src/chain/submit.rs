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
