use bingo_http::{Decode, metered::Clock, stream::{Chunks, model_stream}};
use bingo_sdk::{CancellationToken, ModelEvent, ProviderError};
use futures::{StreamExt, stream};

struct Lines;
impl Decode for Lines {
    fn decode(&mut self, event: &str, data: &str) -> Result<Vec<ModelEvent>, ProviderError> {
        if event == "error" {
            return Err(ProviderError::Stream { message: data.into() });
        }
        Ok(vec![ModelEvent::TextDelta { text: data.into() }])
    }
}

fn body(bytes: &[u8], size: usize) -> Chunks {
    let chunks: Vec<_> = bytes.chunks(size).map(|part| Ok(part.to_vec())).collect();
    Box::pin(stream::iter(chunks))
}

async fn drain(chunks: Chunks) -> Vec<Result<ModelEvent, ProviderError>> {
    model_stream(chunks, Clock::new(), CancellationToken::new(), Lines).collect().await
}

#[tokio::test]
async fn chunked_equals_whole_and_keeps_events_before_an_error() {
    let bytes = b"data: first\n\ndata: second\n\nevent: error\ndata: refused\n\n";
    let whole = drain(body(bytes, bytes.len())).await;
    assert_eq!(whole, drain(body(bytes, 3)).await);
    assert_eq!(whole, vec![
        Ok(ModelEvent::TextDelta { text: "first".into() }),
        Ok(ModelEvent::TextDelta { text: "second".into() }),
        Err(ProviderError::Stream { message: "refused".into() }),
    ]);
}

#[tokio::test(start_paused = true)]
async fn quiet_times_out_and_ends() {
    let mut events = model_stream(Box::pin(stream::pending()), Clock::new(), CancellationToken::new(), Lines);
    assert_eq!(events.next().await, Some(Err(ProviderError::Timeout)));
    assert_eq!(events.next().await, None);
}

#[tokio::test]
async fn cancellation_stops_a_pending_stream() {
    let cancel = CancellationToken::new();
    let mut events = model_stream(Box::pin(stream::pending()), Clock::new(), cancel.clone(), Lines);
    assert!(futures::poll!(events.next()).is_pending());
    cancel.cancel();
    assert_eq!(events.next().await, None);
}

#[tokio::test]
async fn transport_failure_ends() {
    let failure = ProviderError::Transport { message: "connection reset".into() };
    let chunks = Box::pin(stream::iter(vec![Err(failure.clone()), Ok(b"data: unseen\n\n".to_vec())]));
    assert_eq!(drain(chunks).await, vec![Err(failure)]);
}

#[tokio::test]
async fn a_body_that_never_frames_is_an_error() {
    let chunks = Box::pin(stream::iter((0..9).map(|_| Ok(vec![b'x'; 1024 * 1024]))));
    assert!(matches!(drain(chunks).await.last(), Some(Err(ProviderError::Stream { .. }))));
}
