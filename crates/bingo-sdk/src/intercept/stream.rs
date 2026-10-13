//! A shared stream cursor lets a failed wrapper leave its continuation's stream standing.

use crate::{ModelEvent, ModelStream, ProviderError};
use futures::Stream;
use std::pin::Pin;
use std::sync::{Arc, Mutex};
use std::task::{Context, Poll};

#[derive(Clone)]
pub struct StepStream(Arc<Mutex<ModelStream>>);

impl StepStream {
    pub fn new(stream: ModelStream) -> Self {
        Self(Arc::new(Mutex::new(stream)))
    }
}

impl PartialEq for StepStream {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.0, &other.0)
    }
}

impl std::fmt::Debug for StepStream {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("StepStream").finish_non_exhaustive()
    }
}

impl Stream for StepStream {
    type Item = Result<ModelEvent, ProviderError>;
    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        self.0
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .as_mut()
            .poll_next(cx)
    }
}
