//! File stamps and the poll that reloads explicitly named plugin directories.
pub use bingo_plugin_host::stamps::{Stamp, Stamps, changed, stamps};
use std::path::PathBuf;

pub struct Watch {
    root: PathBuf,
    before: Stamps,
    bridge: std::sync::Arc<crate::Bridge>,
    notices: std::sync::Arc<crate::notice::Notices>,
}

impl Watch {
    pub fn new(
        root: PathBuf,
        bridge: std::sync::Arc<crate::Bridge>,
        notices: std::sync::Arc<crate::notice::Notices>,
    ) -> std::io::Result<Self> {
        Ok(Self {
            before: stamps(&root)?,
            root,
            bridge,
            notices,
        })
    }

    pub async fn run(mut self, stop: bingo_sdk::CancellationToken) {
        let mut clock = tokio::time::interval_at(
            tokio::time::Instant::now() + std::time::Duration::from_secs(1),
            std::time::Duration::from_secs(1),
        );
        clock.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
        loop {
            tokio::select! {
                biased;
                _ = stop.cancelled() => return,
                _ = clock.tick() => self.poll().await,
            }
        }
    }

    async fn poll(&mut self) {
        let root = self.root.clone();
        let Ok(Ok(after)) = tokio::task::spawn_blocking(move || stamps(&root)).await else {
            return;
        };
        if !changed(&self.before, &after) {
            return;
        }
        self.before = after;
        if let Some(found) = crate::discovery::read(&self.root, &self.notices) {
            self.bridge.reload(found.manifest).await;
        }
    }
}
