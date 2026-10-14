//! Startup discovery owns no live sessions and is cancelled before timers stop.

use bingo_sdk::HostHandle;
use std::sync::{Arc, Mutex};
use tokio::task::JoinHandle;

use crate::RoomsHook;

#[derive(Debug, Default)]
pub(crate) struct Recovery(Mutex<Option<JoinHandle<()>>>);

impl Recovery {
    pub(crate) fn start(&self, hook: Arc<RoomsHook>, host: HostHandle) {
        let mut task = self.0.lock().unwrap_or_else(|held| held.into_inner());
        if task.is_some() {
            return;
        }
        *task = Some(tokio::spawn(async move {
            if let Err(error) = hook.start(&host).await {
                tracing::warn!(%error, "rooms could not be restored");
            }
        }));
    }

    pub(crate) async fn stop(&self) {
        let task = self
            .0
            .lock()
            .unwrap_or_else(|held| held.into_inner())
            .take();
        if let Some(task) = task {
            task.abort();
            // A restoration cannot arm another timer after the hook clears them.
            let _ = task.await;
        }
    }
}

impl Drop for Recovery {
    fn drop(&mut self) {
        if let Some(task) = self
            .0
            .get_mut()
            .unwrap_or_else(|held| held.into_inner())
            .take()
        {
            task.abort();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bingo_sdk::{Plugin, testing::TestHost};
    use std::sync::atomic::{AtomicBool, Ordering};

    struct Reading(Arc<AtomicBool>);
    impl Drop for Reading {
        fn drop(&mut self) {
            self.0.store(false, Ordering::SeqCst);
        }
    }

    #[tokio::test]
    async fn startup_does_not_wait_for_discovery_and_stop_cancels_it() {
        let host = TestHost::default();
        let fleet = crate::tests::Fleet::default();
        let reader: Arc<dyn bingo_sdk::store::reader::SessionReader> = Arc::new(fleet);
        host.on_service_any(move |key| {
            (key == bingo_sdk::store::reader::KEY.name())
                .then(|| Arc::new(reader.clone()) as Arc<dyn std::any::Any + Send + Sync>)
        });
        let entered = Arc::new(tokio::sync::Notify::new());
        let reading = Arc::new(AtomicBool::new(false));
        let (began, active) = (entered.clone(), reading.clone());
        host.on_sessions(move |_| {
            let (began, active) = (began.clone(), active.clone());
            async move {
                active.store(true, Ordering::SeqCst);
                let _reading = Reading(active);
                began.notify_one();
                std::future::pending().await
            }
        });
        let plugin = crate::RoomsPlugin::default();
        plugin.start(host.handle()).await.unwrap();
        entered.notified().await;
        assert!(reading.load(Ordering::SeqCst));
        plugin.stop().await.unwrap();
        assert!(!reading.load(Ordering::SeqCst), "stop joins cancellation");
    }
}
