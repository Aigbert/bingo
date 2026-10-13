use super::*;

pub(crate) struct Compact;
impl Point for Compact {
    type Input = CompactInput;
    type Output = Result<Compaction, CompactError>;
    const POINT: LinkPoint = LinkPoint::SessionCompact;
    fn dispatch<'a>(
        link: &'a dyn Interceptor,
        cx: &'a HookContext,
        input: Self::Input,
        next: Next<'a, Self::Input, Self::Output>,
    ) -> BoxFuture<'a, Result<Self::Output, KernelError>> {
        Box::pin(link.session_compact(cx, input, next))
    }
}
