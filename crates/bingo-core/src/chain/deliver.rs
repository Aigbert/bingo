//! Delivery has one outbound chain and one inbound chain.
use super::*;

pub(crate) async fn receive(
    input: Input,
    links: &[Link],
    cx: &HookContext,
    report: Report,
) -> Result<Input, KernelError> {
    json::dispatch(
        catalogue::SESSION_RECEIVE,
        links,
        cx,
        input,
        Next::new(|input| Box::pin(async { Ok(input) })),
        report,
    )
    .await
}
