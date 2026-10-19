//! Four active drawings, with only the latest waiter for each site.
use bingo_sdk::render::{Component, RenderRequest};
use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};
use tokio::sync::{OwnedSemaphorePermit, Semaphore, oneshot};

type Key = (Component, String);
#[derive(Default)]
struct Waiting {
    serial: u64,
    entries: BTreeMap<Key, (u64, oneshot::Sender<()>)>,
}
pub(super) struct Queue {
    permits: Arc<Semaphore>,
    waiting: Mutex<Waiting>,
}
impl Default for Queue {
    fn default() -> Self {
        Self {
            permits: Arc::new(Semaphore::new(4)),
            waiting: Mutex::default(),
        }
    }
}
struct Ticket<'a> {
    queue: &'a Queue,
    key: Key,
    serial: u64,
}
impl Drop for Ticket<'_> {
    fn drop(&mut self) {
        let mut waiting = self.queue.waiting.lock().unwrap_or_else(|e| e.into_inner());
        if waiting
            .entries
            .get(&self.key)
            .is_some_and(|(serial, _)| *serial == self.serial)
        {
            waiting.entries.remove(&self.key);
        }
    }
}
impl Queue {
    pub(super) async fn enter(&self, request: &RenderRequest) -> Option<OwnedSemaphorePermit> {
        if let Ok(permit) = self.permits.clone().try_acquire_owned() {
            return Some(permit);
        }
        let (sender, cancelled) = oneshot::channel();
        let key = (request.component, request.request_id.clone());
        let ticket = {
            let mut waiting = self.waiting.lock().unwrap_or_else(|e| e.into_inner());
            waiting.serial = waiting.serial.wrapping_add(1);
            let serial = waiting.serial;
            waiting.entries.insert(key.clone(), (serial, sender));
            Ticket {
                queue: self,
                key,
                serial,
            }
        };
        let result = tokio::select! {
            biased;
            _ = cancelled => None,
            result = self.permits.clone().acquire_owned() => result.ok(),
        };
        drop(ticket);
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bingo_sdk::render::{SurfaceName, Viewport};
    use std::task::Poll;
    #[tokio::test]
    async fn four_run_and_only_the_latest_waiter_for_a_site_survives() {
        let queue = Queue::default();
        let request = RenderRequest {
            component: Component::Pane,
            request_id: "counter".into(),
            surface: SurfaceName::Terminal,
            props: serde_json::json!({}),
            viewport: Viewport {
                glyphs: Default::default(),
                place: None,
                columns: 80,
                rows: 8,
            },
        };
        let mut held = Vec::new();
        for _ in 0..4 {
            held.push(queue.enter(&request).await.unwrap());
        }
        let old = queue.enter(&request);
        tokio::pin!(old);
        assert!(matches!(futures::poll!(&mut old), Poll::Pending));
        let latest = queue.enter(&request);
        tokio::pin!(latest);
        assert!(matches!(futures::poll!(&mut latest), Poll::Pending));
        assert!(old.await.is_none());
        held.pop();
        assert!(latest.await.is_some());
        assert!(queue.waiting.lock().unwrap().entries.is_empty());
    }
}
