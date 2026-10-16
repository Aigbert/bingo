//! File changes shared by both plugin runtimes.
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

/// The directory the host lays author types into. It is the host's output, not
/// the author's module: two bingo versions laying different types would
/// otherwise mark each other's plugins changed every poll, forever.
const HOST_LAID: &str = ".bingo";

pub fn stamps(root: &std::path::Path) -> std::io::Result<Stamps> {
    let mut stamps = Stamps::new();
    let mut directories = vec![root.to_path_buf()];
    while let Some(directory) = directories.pop() {
        for entry in std::fs::read_dir(directory)? {
            let entry = entry?;
            let kind = entry.file_type()?;
            if kind.is_dir() {
                if entry.file_name() != HOST_LAID {
                    directories.push(entry.path());
                }
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

    #[test]
    fn types_the_host_lays_are_not_a_change() {
        let root = tempfile::tempdir().unwrap();
        std::fs::write(root.path().join("register.ts"), "x").unwrap();
        let before = stamps(root.path()).unwrap();
        let laid = root.path().join(".bingo/types/bingo");
        std::fs::create_dir_all(&laid).unwrap();
        std::fs::write(laid.join("events.ts"), "another version").unwrap();
        assert!(!changed(&before, &stamps(root.path()).unwrap()));
        std::fs::write(root.path().join("register.ts"), "xy").unwrap();
        assert!(changed(&before, &stamps(root.path()).unwrap()));
    }
}
