//! Completion text wraps the finished answer and is committed after its terminal frame.
use super::*;

pub(crate) async fn text(
    input: catalogue::Turn,
    links: &[Link],
    cx: &HookContext,
    report: Report,
) -> Result<Option<String>, KernelError> {
    json::dispatch(
        catalogue::TURN_COMPLETE,
        links,
        cx,
        input,
        Next::new(|_| Box::pin(async { Ok(None::<String>) })),
        report,
    )
    .await
}
