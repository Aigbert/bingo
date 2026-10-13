//! The watcher and callback preflight share the bridge's file stamps.
pub use bingo_plugin_host::stamps::{Stamp, Stamps, changed, stamps};
use std::path::PathBuf;

pub struct Watch {
    bridge: std::sync::Arc<crate::Bridge>,
}
impl Watch {
    pub fn new(
        root: PathBuf,
        bridge: std::sync::Arc<crate::Bridge>,
        _notices: std::sync::Arc<crate::notice::Notices>,
    ) -> std::io::Result<Self> {
        bridge.watch(stamps(&root)?);
        Ok(Self { bridge })
    }
    pub async fn run(self, stop: bingo_sdk::CancellationToken) {
        let mut clock = tokio::time::interval_at(
            tokio::time::Instant::now() + std::time::Duration::from_secs(1),
            std::time::Duration::from_secs(1),
        );
        clock.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
        loop {
            tokio::select! {
                biased;
                _ = stop.cancelled() => return,
                _ = clock.tick() => self.bridge.refresh().await,
            }
        }
    }
}
