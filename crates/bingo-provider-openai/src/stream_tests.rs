use crate::events::Decoder;
use bingo_http::{
    metered::Clock,
    stream::{Chunks, model_stream},
};
use bingo_sdk::{CancellationToken, ModelEvent, ProviderError};
use bingo_sdk::{FinishReason, UnifiedFinish};
use futures::StreamExt;
use futures::stream;

/// A fixture body, cut into small chunks so every frame straddles one.
fn fixture(name: &str, chunk: usize) -> Chunks {
    let body = std::fs::read(crate::tests::fixture(name)).expect("read the fixture");
    let parts: Vec<Result<Vec<u8>, ProviderError>> =
        body.chunks(chunk).map(|c| Ok(c.to_vec())).collect();
    Box::pin(stream::iter(parts))
}

async fn drain(chunks: Chunks) -> Vec<Result<ModelEvent, ProviderError>> {
    model_stream(
        chunks,
        Clock::new(),
        CancellationToken::new(),
        Decoder::new(),
    )
    .collect()
    .await
}

#[tokio::test]
async fn a_chunked_body_decodes_to_the_same_events_as_one_piece() {
    let whole = drain(fixture("text.sse", 4096)).await;
    let split = drain(fixture("text.sse", 7)).await;
    assert_eq!(whole, split, "framing must not depend on chunk size");
    assert!(matches!(
        whole.last(),
        Some(Ok(ModelEvent::Finish {
            finish_reason: FinishReason {
                unified: UnifiedFinish::Stop,
                ..
            },
            ..
        }))
    ));
}

#[tokio::test]
async fn a_mid_stream_failure_arrives_after_everything_that_preceded_it() {
    let items = drain(fixture("failed.sse", 13)).await;
    let (last, before) = items.split_last().expect("some events");
    assert!(
        matches!(last, Err(ProviderError::Server { status: 500, .. })),
        "{last:?}"
    );
    assert!(before.iter().all(Result::is_ok));
    assert!(matches!(
        before.last(),
        Some(Ok(ModelEvent::TextDelta { .. }))
    ));
}

#[tokio::test]
async fn a_cancelled_turn_stops_the_stream_before_it_finishes() {
    let cancel = CancellationToken::new();
    let mut events = model_stream(
        fixture("text.sse", 64),
        Clock::new(),
        cancel.clone(),
        Decoder::new(),
    );
    assert!(events.next().await.is_some());
    cancel.cancel();
    let rest: Vec<_> = events.collect().await;
    assert!(
        !rest
            .iter()
            .any(|e| matches!(e, Ok(ModelEvent::Finish { .. }))),
        "a cancelled stream never finishes"
    );
}

#[tokio::test]
async fn a_token_cancelled_before_the_first_poll_yields_nothing() {
    let cancel = CancellationToken::new();
    cancel.cancel();
    let events: Vec<_> = model_stream(
        fixture("text.sse", 64),
        Clock::new(),
        cancel,
        Decoder::new(),
    )
    .collect()
    .await;
    assert!(events.is_empty());
}
