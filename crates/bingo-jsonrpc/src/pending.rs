use crate::PeerError;
use crate::codec::{Id, Outcome, Response};
use serde_json::Value;
use std::collections::HashMap;
use std::sync::{Arc, Mutex, MutexGuard};
use tokio::sync::oneshot;

type Reply = Result<Value, PeerError>;

#[derive(Default)]
pub(crate) struct State {
    pub error: Option<PeerError>,
    pub waiting: HashMap<Id, oneshot::Sender<Reply>>,
}

#[derive(Clone, Default)]
pub(crate) struct Pending(Arc<Mutex<State>>);

impl Pending {
    pub fn lock(&self) -> MutexGuard<'_, State> {
        self.0.lock().unwrap_or_else(|poison| poison.into_inner())
    }

    pub fn register(&self, id: Id) -> Result<(Guard, oneshot::Receiver<Reply>), PeerError> {
        let mut state = self.lock();
        if let Some(error) = &state.error {
            return Err(error.clone());
        }
        let (tx, rx) = oneshot::channel();
        state.waiting.insert(id.clone(), tx);
        Ok((
            Guard {
                pending: self.clone(),
                id,
            },
            rx,
        ))
    }

    pub fn settle(&self, response: Response) -> Option<Response> {
        let Some(sender) = response
            .id
            .as_ref()
            .and_then(|id| self.lock().waiting.remove(id))
        else {
            return Some(response);
        };
        let _ = sender.send(match response.outcome {
            Outcome::Result(value) => Ok(value),
            Outcome::Error(error) => Err(PeerError::Remote(error)),
        });
        None
    }

    pub fn close(&self, error: PeerError) -> bool {
        let mut state = self.lock();
        if state.error.is_some() {
            return false;
        }
        state.error = Some(error.clone());
        for (_, sender) in state.waiting.drain() {
            let _ = sender.send(Err(error.clone()));
        }
        true
    }
}

pub(crate) struct Guard {
    pending: Pending,
    id: Id,
}

impl Drop for Guard {
    fn drop(&mut self) {
        self.pending.lock().waiting.remove(&self.id);
    }
}
