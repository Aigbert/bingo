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

/// A model change may also name a provider; an absent provider stays untouched.
pub(crate) async fn change(
    host: &Host,
    session: &SessionId,
    change: SessionChange,
) -> Result<SessionChange, KernelError> {
    Ok(match change {
        SessionChange::Key(key) => SessionChange::Key(value(host, session, "key", key).await?),
        SessionChange::Title(title) => {
            SessionChange::Title(value(host, session, "title", title).await?)
        }
        SessionChange::Thinking(effort) => {
            SessionChange::Thinking(value(host, session, "thinking", effort).await?)
        }
        SessionChange::Model { provider, model } => {
            let provider = match provider {
                Some(provider) => Some(value(host, session, "provider", provider).await?),
                None => None,
            };
            let model = value(host, session, "model", model).await?;
            SessionChange::Model { provider, model }
        }
    })
}

async fn value<T: serde::Serialize + serde::de::DeserializeOwned>(
    host: &Host,
    session: &SessionId,
    key: &str,
    value: T,
) -> Result<T, KernelError> {
    let value = serde_json::to_value(value).map_err(invalid)?;
    serde_json::from_value(set(host, session, key, value).await?).map_err(invalid)
}
fn invalid(error: impl std::fmt::Display) -> KernelError {
    KernelError::new(ErrorCode::InvalidInput, error.to_string())
}
