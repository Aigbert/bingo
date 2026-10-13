//! Independent cursors preserve the continuation when a stream wrapper fails.
use crate::{ModelEvent, ModelStream, ProviderError};
use futures::Stream;
use std::collections::{BTreeMap, VecDeque};
use std::pin::Pin;
use std::sync::{Arc, Mutex};
use std::task::{Context, Poll};

type Entry = Result<ModelEvent, ProviderError>;
struct Replay {
    stream: ModelStream,
    buffered: VecDeque<Entry>,
    start: usize,
    cursors: BTreeMap<usize, usize>,
    serial: usize,
}

pub struct StepStream {
    replay: Arc<Mutex<Replay>>,
    cursor: usize,
}
impl StepStream {
    pub fn new(stream: ModelStream) -> Self {
        Self {
            replay: Arc::new(Mutex::new(Replay {
                stream,
                buffered: VecDeque::new(),
                start: 0,
                cursors: BTreeMap::from([(0, 0)]),
                serial: 0,
            })),
            cursor: 0,
        }
    }
}
impl Clone for StepStream {
    fn clone(&self) -> Self {
        let mut replay = self.replay.lock().unwrap_or_else(|e| e.into_inner());
        let position = replay.cursors[&self.cursor];
        replay.serial += 1;
        let cursor = replay.serial;
        replay.cursors.insert(cursor, position);
        Self {
            replay: self.replay.clone(),
            cursor,
        }
    }
}
impl Drop for StepStream {
    fn drop(&mut self) {
        let mut replay = self.replay.lock().unwrap_or_else(|e| e.into_inner());
        replay.cursors.remove(&self.cursor);
        replay.prune();
    }
}
impl PartialEq for StepStream {
    fn eq(&self, other: &Self) -> bool {
        if !Arc::ptr_eq(&self.replay, &other.replay) {
            return false;
        }
        let replay = self.replay.lock().unwrap_or_else(|e| e.into_inner());
        replay.cursors.get(&self.cursor) == replay.cursors.get(&other.cursor)
    }
}
impl std::fmt::Debug for StepStream {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("StepStream").finish_non_exhaustive()
    }
}
impl Stream for StepStream {
    type Item = Entry;
    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Entry>> {
        let mut replay = self.replay.lock().unwrap_or_else(|e| e.into_inner());
        let position = replay.cursors[&self.cursor];
        let entry = if let Some(entry) = replay.buffered.get(position - replay.start) {
            entry.clone()
        } else {
            match replay.stream.as_mut().poll_next(cx) {
                Poll::Ready(Some(entry)) => {
                    replay.buffered.push_back(entry.clone());
                    entry
                }
                other => return other,
            }
        };
        replay.cursors.insert(self.cursor, position + 1);
        replay.prune();
        Poll::Ready(Some(entry))
    }
}
impl Replay {
    fn prune(&mut self) {
        let keep = self
            .cursors
            .values()
            .copied()
            .min()
            .unwrap_or(self.start + self.buffered.len());
        self.buffered.drain(..keep - self.start);
        self.start = keep;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use futures::StreamExt;
    #[tokio::test]
    async fn a_failed_wrapper_cannot_consume_the_continuations_events() {
        let event = Ok(ModelEvent::TextStart { id: "one".into() });
        let mut original = StepStream::new(Box::pin(futures::stream::iter([event.clone()])));
        let mut wrapper = original.clone();
        assert_eq!(wrapper.next().await, Some(event.clone()));
        assert_ne!(wrapper, original);
        drop(wrapper);
        assert_eq!(original.next().await, Some(event));
        assert_eq!(original.next().await, None);
    }
}
