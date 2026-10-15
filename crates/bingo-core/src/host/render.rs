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
    let id = request.request_id.clone();
    let result = crate::chain::render::drawn(
        &host.hook_set().links().await,
        &host.catalogue_context(Some(session)),
        request,
        report.clone(),
    )
    .await;
    for notice in report.take() {
        let _ = mailbox.record(notice).await;
    }
    let (view, drawn) = match result {
        Ok(drawn) => drawn,
        Err(error) => {
            live.drawings.record(&id, None);
            return Err(error);
        }
    };
    live.drawings.record(&id, drawn);
    Ok(view)
}

pub(super) async fn invalidate(
    host: &Host,
    session: &SessionId,
    requests: Vec<String>,
) -> Result<(), KernelError> {
    let Some(live) = host.lock().get(session).cloned() else {
        return Ok(());
    };
    let mailbox = live.mailbox.clone();
    let mut pending: Vec<_> = requests
        .into_iter()
        .collect::<std::collections::BTreeSet<_>>()
        .into_iter()
        .collect();
    while !pending.is_empty() {
        let (accepted, delay) = {
            let mut throttle = live.invalidations.lock().unwrap_or_else(|e| e.into_inner());
            let now = live.render_started.elapsed();
            let mut accepted = Vec::new();
            pending.retain(|request| {
                if throttle.admit(request, now) {
                    accepted.push(request.clone());
                    false
                } else {
                    true
                }
            });
            let delay = pending
                .iter()
                .map(|request| throttle.delay(request, now))
                .min();
            (accepted, delay)
        };
        if !accepted.is_empty() {
            mailbox.invalidate(accepted);
        }
        if let Some(delay) = delay {
            tokio::time::sleep(delay).await;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[tokio::test]
    async fn invalidation_never_opens_a_session_that_is_not_live() {
        let dir = tempfile::tempdir().unwrap();
        let host = Host::build(vec![], HostConfig::new(Env::rooted(dir.path())))
            .await
            .unwrap();
        host.invalidate(&SessionId::from_raw("closed"), vec!["pane".into()])
            .await
            .unwrap();
        assert!(host.lock().is_empty());
    }
}
