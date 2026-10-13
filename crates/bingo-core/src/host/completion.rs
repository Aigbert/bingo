//! Side completions share one grant check and one catalogue chain.
use super::*;

pub(super) async fn complete(
    host: &Host,
    session: &SessionId,
    turn: Option<&TurnId>,
    request: Completion,
) -> Result<Vec<ModelEvent>, KernelError> {
    if !catalog::model_granted(host, &request.plugin).await {
        return Err(KernelError::new(
            ErrorCode::PermissionDenied,
            format!("{} needs the model grant", request.plugin),
        ));
    }
    let mailbox = host.mailbox_of(session).await?;
    let mut cx = host.catalogue_context(Some(session));
    cx.turn = turn.cloned();
    let report = crate::chain::Report::default();
    let result = crate::chain::json::dispatch(
        catalogue::MODEL_COMPLETE,
        &host.hook_set().links().await,
        &cx,
        catalogue::Complete {
            plugin: request.plugin,
            request: request.request,
        },
        Next::new(|request: catalogue::Complete| {
            Box::pin(async {
                mailbox
                    .complete(
                        turn.cloned(),
                        Completion {
                            plugin: request.plugin,
                            request: request.request,
                        },
                    )
                    .await
            })
        }),
        report.clone(),
    )
    .await;
    for notice in report.take() {
        let _ = mailbox.record(notice).await;
    }
    result
}
