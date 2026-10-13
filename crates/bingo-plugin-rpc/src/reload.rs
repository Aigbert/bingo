//! File stamps and the poll that reloads explicitly named plugin directories.
use std::collections::BTreeMap;
use std::path::PathBuf;
use std::time::SystemTime;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Stamp {
    pub len: u64,
    pub modified: SystemTime,
}
pub type Stamps = BTreeMap<PathBuf, Stamp>;

pub fn changed(before: &Stamps, after: &Stamps) -> bool {
    before != after
}

pub fn stamps(root: &std::path::Path) -> std::io::Result<Stamps> {
    let mut stamps = Stamps::new();
    let mut directories = vec![root.to_path_buf()];
    while let Some(directory) = directories.pop() {
        for entry in std::fs::read_dir(directory)? {
            let entry = entry?;
            let kind = entry.file_type()?;
            if kind.is_dir() {
                directories.push(entry.path());
            } else if kind.is_file() {
                let metadata = entry.metadata()?;
                stamps.insert(
                    entry.path(),
                    Stamp {
                        len: metadata.len(),
                        modified: metadata.modified()?,
                    },
                );
            }
        }
    }
    Ok(stamps)
}

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

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn every_stamp_dimension_and_deletion_are_changes() {
        let before = Stamps::from([(
            PathBuf::from("plugin.json"),
            Stamp {
                len: 1,
                modified: SystemTime::UNIX_EPOCH,
            },
        )]);
        assert!(!changed(&before, &before));
        assert!(changed(&before, &Stamps::new()));
        let mut after = before.clone();
        after.get_mut(&PathBuf::from("plugin.json")).unwrap().len = 2;
        assert!(changed(&before, &after));
        let mut after = before.clone();
        after
            .get_mut(&PathBuf::from("plugin.json"))
            .unwrap()
            .modified += std::time::Duration::from_secs(1);
        assert!(changed(&before, &after));
        let after = Stamps::from([(
            PathBuf::from("main.py"),
            before.values().next().unwrap().clone(),
        )]);
        assert!(changed(&before, &after));
    }
}
