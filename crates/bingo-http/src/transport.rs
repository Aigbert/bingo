//! A metered upload and the wait for its response head.
use crate::metered::{Clock, meter, quiet};
use bingo_sdk::ProviderError;
use std::time::Duration;

pub async fn round_trip(
    builder: reqwest::RequestBuilder,
    idle: Duration,
) -> Result<(reqwest::Response, Clock), ProviderError> {
    let clock = Clock::new();
    let (client, request) = meter(builder, &clock)?;
    let sent = tokio::select! {
        biased;
        sent = client.execute(request) => sent,
        () = quiet(&clock, idle) => return Err(ProviderError::Timeout),
    };
    let response = sent.map_err(|e| ProviderError::Transport {
        message: e.to_string(),
    })?;
    // The response head is activity too; its body gets the whole idle bound.
    clock.stamp();
    Ok((response, clock))
}
