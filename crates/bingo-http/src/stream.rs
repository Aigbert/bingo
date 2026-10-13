//! The streaming body: HTTP chunks → SSE frames → `ModelEvent`s.
//!
//! Two guards sit on it. Silence ends the stream as a `Timeout` — silence
//! being that no byte has moved on the wire, in either direction, for
//! [`IDLE_TIMEOUT`]; the clock is the one the request's own body stamped on
//! its way out (ADR-0056 §2) — so a server that connects and then goes quiet
//! cannot hang a headless run. A cancelled token ends the stream silently,
//! which is how an interrupt reaches the wire. Neither retries — the turn
//! loop owns the ladder.
//!
//! [`crate::CONNECT_TIMEOUT`] bounds the phase before either of them: reaching the
//! server at all. Past it, size never ends a request; only silence does.
//!
//! Provider decoders own the vocabulary; this module owns the transport.

use std::collections::VecDeque;
use std::pin::Pin;

use bingo_sdk::{CancellationToken, ModelEvent, ModelStream, ProviderError};
use futures::{Stream, StreamExt};

use crate::Decode;
use crate::metered::{Clock, quiet};
use crate::sse::{SseFrame, SseParser};

use crate::IDLE_TIMEOUT;

/// The response body as bytes, with transport failures already named. Erasing
/// the chunk type here is what lets a fixture drive the decoding half without
/// an HTTP server.
pub type Chunks = Pin<Box<dyn Stream<Item = Result<Vec<u8>, ProviderError>> + Send>>;

pub fn chunks(response: reqwest::Response) -> Chunks {
    Box::pin(response.bytes_stream().map(|chunk| {
        chunk
            .map(|bytes| bytes.to_vec())
            .map_err(|e| ProviderError::Transport {
                message: e.to_string(),
            })
    }))
}

pub fn model_stream<D: Decode>(
    chunks: Chunks,
    clock: Clock,
    cancel: CancellationToken,
    decoder: D,
) -> ModelStream {
    let body = Body {
        chunks,
        parser: SseParser::new(),
        decoder,
        queue: VecDeque::new(),
        failure: None,
        done: false,
        clock,
        cancel,
    };
    Box::pin(futures::stream::unfold(body, |mut body| async move {
        body.next().await.map(|item| (item, body))
    }))
}

/// One response being read. `queue` holds events already decoded but not yet
/// handed out, so a chunk carrying several frames is delivered event by event
/// and a failure still arrives *after* everything that preceded it.
struct Body<D> {
    chunks: Chunks,
    parser: SseParser,
    decoder: D,
    queue: VecDeque<ModelEvent>,
    failure: Option<ProviderError>,
    done: bool,
    /// The request stamped it too: one connection, one silence.
    clock: Clock,
    cancel: CancellationToken,
}

/// What ended one wait on the body.
enum Pulled {
    Cancelled,
    Chunk(Option<Result<Vec<u8>, ProviderError>>),
    Silent,
}

impl<D: Decode> Body<D> {
    async fn next(&mut self) -> Option<Result<ModelEvent, ProviderError>> {
        loop {
            if let Some(event) = self.queue.pop_front() {
                return Some(Ok(event));
            }
            if let Some(failure) = self.failure.take() {
                self.done = true;
                return Some(Err(failure));
            }
            if self.done || self.cancel.is_cancelled() {
                return None;
            }
            self.pump().await;
        }
    }

    /// One chunk, decoded into the queue — or the thing that ends the stream.
    async fn pump(&mut self) {
        match self.pull().await {
            // A cancelled turn ends where it stands; the loop knows why.
            Pulled::Cancelled => self.done = true,
            Pulled::Silent => self.fail(ProviderError::Timeout),
            Pulled::Chunk(None) => self.end(),
            Pulled::Chunk(Some(Err(transport))) => self.fail(transport),
            Pulled::Chunk(Some(Ok(bytes))) => self.feed(&bytes),
        }
    }

    /// Whichever comes first: the interrupt, the next chunk, the silence. A
    /// chunk that is ready beats a guard that has just expired.
    async fn pull(&mut self) -> Pulled {
        let cancel = self.cancel.clone();
        let clock = self.clock.clone();
        tokio::select! {
            biased;
            _ = cancel.cancelled() => Pulled::Cancelled,
            chunk = self.chunks.next() => {
                clock.stamp();
                Pulled::Chunk(chunk)
            }
            () = quiet(&clock, IDLE_TIMEOUT) => Pulled::Silent,
        }
    }

    fn feed(&mut self, bytes: &[u8]) {
        match self.parser.feed(bytes) {
            Ok(frames) => {
                for frame in frames {
                    if !self.absorb(&frame) {
                        return;
                    }
                }
            }
            Err(message) => self.fail(ProviderError::Stream { message }),
        }
    }

    /// The body ended: read whatever the last frame left unterminated.
    fn end(&mut self) {
        self.done = true;
        if let Some(frame) = self.parser.finish() {
            self.absorb(&frame);
        }
    }

    /// False when this frame ended the stream.
    fn absorb(&mut self, frame: &SseFrame) -> bool {
        match self.decoder.decode(&frame.event, &frame.data) {
            Ok(events) => {
                self.queue.extend(events);
                true
            }
            Err(error) => {
                self.fail(error);
                false
            }
        }
    }

    fn fail(&mut self, error: ProviderError) {
        self.failure = Some(error);
    }
}
