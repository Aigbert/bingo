//! A proposed setting passes its chain before callers reconfigure or persist it.
use crate::host::Host;
use bingo_sdk::*;
use serde_json::Value;

pub(crate) async fn set(
    host: &Host,
    session: &SessionId,
    key: &str,
    value: Value,
) -> Result<Value, KernelError> {
    let input = catalogue::ConfigSet {
        key: key.into(),
        value,
    };
    let report = crate::chain::Report::default();
    let result = crate::chain::json::dispatch(
        catalogue::CONFIG_SET,
        &host.hook_set().links().await,
        &host.catalogue_context(Some(session)),
        input,
        Next::new(|input| Box::pin(async { Ok(input) })),
        report.clone(),
    )
    .await;
    for notice in report.take() {
        if let ItemBody::Notice { level, code, text } = notice {
            let _ = host.notice(level, &code, &text).await;
        }
    }
    result.map(|set: catalogue::ConfigSet| set.value)
}
