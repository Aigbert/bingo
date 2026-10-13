//! Only a row's owner may rewrite it; other links may observe it.
use super::*;

pub(crate) async fn rows(
    input: catalogue::Append,
    links: &[Link],
    cx: &HookContext,
    report: Report,
) -> Result<catalogue::Append, KernelError> {
    let links: Vec<_> = links
        .iter()
        .cloned()
        .map(|mut link| {
            let owner = link
                .external
                .as_deref()
                .unwrap_or_else(|| link.interceptor.id());
            if owner != input.plugin {
                link.tier = Tier::Tighten;
            }
            link
        })
        .collect();
    json::dispatch(
        catalogue::SESSION_APPEND,
        &links,
        cx,
        input,
        Next::new(|input| Box::pin(async { Ok(input) })),
        report,
    )
    .await
}
