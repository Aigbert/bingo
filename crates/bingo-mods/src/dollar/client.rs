//! A lazy client attachment owns one fold and the task that advances it.
use bingo_sdk::*;
use futures::StreamExt;
use std::{
    collections::BTreeMap,
    sync::{Arc, Mutex},
};

#[derive(Default)]
pub struct Sessions(tokio::sync::Mutex<BTreeMap<SessionId, Arc<Live>>>);
pub struct Live {
    state: tokio::sync::watch::Receiver<SessionState>,
    pub handle: SessionHandle,
    task: Mutex<Option<tokio::task::JoinHandle<()>>>,
}
impl Live {
    pub fn snapshot(&self) -> SessionState {
        self.state.borrow().clone()
    }
    pub async fn signal(&self, key: &str, value: &serde_json::Value) -> Result<(), String> {
        let mut state = self.state.clone();
        state
            .wait_for(|state| {
                let held = state
                    .signals
                    .get("bingo.mods")
                    .and_then(|values| values.get(key));
                if value.is_null() {
                    held.is_none()
                } else {
                    held.map(|signal| signal.payload()) == Some(value)
                }
            })
            .await
            .map(|_| ())
            .map_err(|_| "session stream closed".into())
    }
    async fn stop(&self) {
        let task = self.task.lock().unwrap_or_else(|e| e.into_inner()).take();
        if let Some(task) = task {
            task.abort();
            let _ = task.await;
        }
    }
}
impl Drop for Live {
    fn drop(&mut self) {
        if let Some(task) = self.task.lock().unwrap_or_else(|e| e.into_inner()).take() {
            task.abort();
        }
    }
}
impl Sessions {
    pub async fn get(&self, host: &HostHandle, id: &SessionId) -> Result<Arc<Live>, String> {
        let mut sessions = self.0.lock().await;
        if let Some(live) = sessions.get(id) {
            return Ok(live.clone());
        }
        let attachment = attach(host, id).await?;
        let (sender, state) = tokio::sync::watch::channel(attachment.snapshot);
        let task = tokio::spawn(fold(sender, attachment.events, host.clone(), id.clone()));
        let live = Arc::new(Live {
            state,
            handle: attachment.handle,
            task: Mutex::new(Some(task)),
        });
        sessions.insert(id.clone(), live.clone());
        Ok(live)
    }
    pub async fn remove(&self, id: &SessionId) {
        let live = self.0.lock().await.remove(id);
        if let Some(live) = live {
            live.stop().await;
        }
    }
    pub async fn stop(&self) {
        let sessions = std::mem::take(&mut *self.0.lock().await);
        for live in sessions.into_values() {
            live.stop().await;
        }
    }
}
async fn attach(host: &HostHandle, id: &SessionId) -> Result<Attachment, String> {
    host.open(
        SessionSelector::ById { id: id.clone() },
        ClientIdentity {
            name: "mods".into(),
            surface: "mods".into(),
        },
        OpenOptions::default(),
    )
    .await
    .map_err(|error| error.to_string())
}
async fn fold(
    state: tokio::sync::watch::Sender<SessionState>,
    mut events: FrameStream,
    host: HostHandle,
    id: SessionId,
) {
    while let Some(frame) = events.next().await {
        let mut applied = Applied::Nothing;
        state.send_modify(|state| applied = state.apply(&frame));
        if matches!(applied, Applied::Lagged) {
            let Ok(attachment) = attach(&host, &id).await else {
                break;
            };
            state.send_replace(attachment.snapshot);
            events = attachment.events;
        }
    }
}
