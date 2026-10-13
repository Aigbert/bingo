//! Drawing runs beside the actor; only notices and invalidations enter it.
use super::*;
use bingo_sdk::render::RenderRequest;

pub(super) async fn render(
    host: &Host,
    session: &SessionId,
    request: RenderRequest,
) -> Result<View, KernelError> {
    let mailbox = host.mailbox_of(session).await?;
    let live = host.live(session)?;
    let Some(_permit) = live.render.enter(&request).await else {
        return Ok(request.engine());
    };
    let report = crate::chain::Report::default();
    let result = crate::chain::render::dispatch(
        &host.hook_set().links().await,
        &host.catalogue_context(Some(session)),
        request,
        report.clone(),
    )
    .await;
    for notice in report.take() {
        let _ = mailbox.record(notice).await;
    }
    result
}

pub(super) async fn invalidate(
    host: &Host,
    session: &SessionId,
    requests: Vec<String>,
) -> Result<(), KernelError> {
    let mailbox = host.mailbox_of(session).await?;
    let live = host.live(session)?;
    let accepted = {
        let mut throttle = live.invalidations.lock().unwrap_or_else(|e| e.into_inner());
        let now = live.render_started.elapsed();
        requests
            .into_iter()
            .collect::<std::collections::BTreeSet<_>>()
            .into_iter()
            .filter(|request| throttle.admit(request, now))
            .collect::<Vec<_>>()
    };
    if !accepted.is_empty() {
        mailbox.invalidate(accepted);
    }
    Ok(())
}
