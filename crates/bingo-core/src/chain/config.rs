//! Config descriptions are shaped before clients see them.
use super::*;

pub(crate) async fn describe(
    config: ConfigView,
    links: &[Link],
    cx: &HookContext,
    report: Report,
) -> Result<ConfigView, KernelError> {
    json::dispatch(
        catalogue::CONFIG_DESCRIBE,
        links,
        cx,
        config,
        Next::new(|config| Box::pin(async { Ok(config) })),
        report,
    )
    .await
}
