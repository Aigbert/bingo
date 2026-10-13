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

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn every_stamp_dimension_and_deletion_are_changes() {
        let before = Stamps::from([(PathBuf::from("plugin.json"), Stamp {len:1, modified:SystemTime::UNIX_EPOCH})]);
        assert!(!changed(&before, &before));
        assert!(changed(&before, &Stamps::new()));
        let mut after = before.clone();
        after.get_mut(&PathBuf::from("plugin.json")).unwrap().len = 2;
        assert!(changed(&before, &after));
        let mut after = before.clone();
        after.get_mut(&PathBuf::from("plugin.json")).unwrap().modified += std::time::Duration::from_secs(1);
        assert!(changed(&before, &after));
        let after = Stamps::from([(PathBuf::from("main.py"), before.values().next().unwrap().clone())]);
        assert!(changed(&before, &after));
    }
}
