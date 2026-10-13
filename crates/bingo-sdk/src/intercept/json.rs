//! The typed API and the JSON API share one continuation.
use crate::*;
use futures::{FutureExt, StreamExt};
use serde::{Serialize, de::DeserializeOwned};
use serde_json::Value;
use std::sync::{Arc, Mutex};

pub fn encode<T: Serialize>(value: T) -> Result<Value, KernelError> {
    serde_json::to_value(value).map_err(failed)
}
pub fn decode<T: DeserializeOwned>(value: Value) -> Result<T, KernelError> {
    serde_json::from_value(value).map_err(failed)
}
fn failed(error: impl std::fmt::Display) -> KernelError {
    KernelError::new(
        ErrorCode::InvalidInput,
        format!("invalid event shape: {error}"),
    )
}

pub async fn bridge<I, O>(
    link: &(impl Interceptor + ?Sized),
    entry: LinkPoint,
    cx: &HookContext,
    input: I,
    next: Next<'_, I, O>,
) -> Result<O, KernelError>
where
    I: Serialize + DeserializeOwned + Send,
    O: Serialize + DeserializeOwned + Send,
{
    let original = encode(input)?;
    let before = original.clone();
    let continuation = Next::at(next.tier(), move |value| {
        async move {
            if let Some(field) = catalogue::pinned_changed(&before, &value, entry.pinned) {
                return Err(failed(format!("pinned field {field}")));
            }
            encode(next.run(decode(value)?).await?)
        }
        .boxed()
    });
    decode(link.chain(entry, cx, original, continuation).await?)
}

pub async fn tool<O>(
    link: &(impl Interceptor + ?Sized),
    entry: LinkPoint,
    cx: &HookContext,
    input: ToolCall,
    next: Next<'_, ToolCall, O>,
) -> Result<O, KernelError>
where
    O: Serialize + DeserializeOwned + Send,
{
    let continuation = Next::at(next.tier(), move |call: catalogue::ToolCall| {
        next.run(call.into()).boxed()
    });
    bridge(
        link,
        entry,
        cx,
        catalogue::ToolCall::try_from(input)?,
        continuation,
    )
    .await
}

pub async fn step(
    link: &(impl Interceptor + ?Sized),
    cx: &HookContext,
    input: ModelRequest,
    next: Next<'_, ModelRequest, Result<StepStream, ProviderError>>,
) -> Result<Result<StepStream, ProviderError>, KernelError> {
    let held = Arc::new(Mutex::new(None));
    let hold = held.clone();
    let continuation = Next::at(next.tier(), move |request| {
        async move {
            next.run_then(request, |original| async move {
                let events: Result<Vec<ModelEvent>, ProviderError> = match original.clone() {
                    Ok(stream) => stream.collect::<Vec<_>>().await.into_iter().collect(),
                    Err(error) => Err(error),
                };
                *hold.lock().unwrap_or_else(|e| e.into_inner()) = Some((events.clone(), original));
                events
            })
            .await
        }
        .boxed()
    });
    let result: Result<Vec<ModelEvent>, ProviderError> =
        bridge(link, catalogue::TURN_STEP, cx, input, continuation).await?;
    let original = held.lock().unwrap_or_else(|e| e.into_inner()).take();
    if let Some((events, original)) = original
        && result == events
    {
        return Ok(original);
    }
    Ok(result
        .map(|events| StepStream::new(Box::pin(futures::stream::iter(events.into_iter().map(Ok))))))
}
