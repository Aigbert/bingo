//! The store on disk: `<data_dir>/schedules/<id>.json`, rebuilt from the
//! directory on every read (ADR-0019 §1). There is no index to drift — `ls`
//! and `rm` work on it — and a file nobody can parse costs one entry and is
//! said out loud, never silently skipped.
//!
//! The reads are synchronous: a permission card's `preview` is a synchronous
//! call, and one shelf of small files read two ways would be two
//! representations of one store.

use std::path::{Path, PathBuf};

pub use bingo_shelf::Unreadable;
use bingo_shelf::{Codec, CodecError, Loaded};

use crate::entry::Entry;

pub const DIRECTORY: &str = "schedules";
pub type Shelf = Loaded<Entry>;

pub trait ScheduleShelf {
    fn ids(&self) -> Vec<&str>;
}

impl ScheduleShelf for Shelf {
    fn ids(&self) -> Vec<&str> {
        self.entries.iter().map(|entry| entry.id.as_str()).collect()
    }
}

#[derive(Debug)]
struct Json;

impl Codec for Json {
    type Record = Entry;
    const EXT: &'static str = "json";

    fn encode(entry: &Entry) -> Result<String, CodecError> {
        entry.document().map_err(|e| CodecError(e.to_string()))
    }

    fn decode(id: &str, text: &str) -> Result<Entry, CodecError> {
        let mut entry: Entry = serde_json::from_str(text).map_err(|e| CodecError(e.to_string()))?;
        entry.id = id.to_string();
        Ok(entry)
    }
}

#[derive(Debug)]
pub struct Store {
    shelf: bingo_shelf::Shelf<Json>,
}

impl Store {
    pub fn new(data_dir: &Path) -> Self {
        Self {
            shelf: bingo_shelf::Shelf::new(data_dir.join(DIRECTORY)),
        }
    }

    pub fn dir(&self) -> &Path {
        self.shelf.dir()
    }
    pub fn path(&self, id: &str) -> PathBuf {
        self.shelf.path(id)
    }
    pub fn load(&self) -> Shelf {
        self.shelf.load()
    }
    pub fn save(&self, entry: &Entry) -> std::io::Result<PathBuf> {
        self.shelf.save(&entry.id, entry)
    }
    pub fn delete(&self, id: &str) -> std::io::Result<()> {
        self.shelf.delete(id)
    }
    pub fn mint(&self) -> String {
        self.shelf.mint()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::entry::tests::entry;

    struct Data {
        home: tempfile::TempDir,
    }

    impl Data {
        fn new() -> Self {
            Self {
                home: tempfile::tempdir().expect("a temp home"),
            }
        }

        fn store(&self) -> Store {
            Store::new(self.home.path())
        }
    }

    fn files(dir: &Path) -> Vec<String> {
        let mut names: Vec<String> = std::fs::read_dir(dir)
            .expect("a directory")
            .flatten()
            .map(|f| f.file_name().to_string_lossy().into_owned())
            .collect();
        names.sort();
        names
    }

    #[test]
    fn an_entry_round_trips_through_the_directory_under_its_id() {
        let data = Data::new();
        let store = data.store();
        let written = entry();
        let path = store.save(&written).expect("saved");
        assert_eq!(path.file_name().expect("a name"), "abcd1234.json");
        assert_eq!(
            path.parent(),
            Some(data.home.path().join("schedules").as_path())
        );

        let shelf = store.load();
        assert_eq!(shelf.entries, [written]);
        assert!(shelf.unreadable.is_empty());
        assert_eq!(
            files(store.dir()),
            ["abcd1234.json"],
            "no tmp file survives"
        );
    }

    #[test]
    fn a_file_nobody_can_parse_costs_one_entry_and_is_named() {
        let data = Data::new();
        let store = data.store();
        store.save(&entry()).expect("saved");
        std::fs::write(store.dir().join("broken.json"), "{ not json at all\n").expect("wrote");
        std::fs::write(store.dir().join("cron.json"), r#"{"spec":"* * * * *"}"#).expect("wrote");
        std::fs::write(store.dir().join("runner.lock"), "4242").expect("wrote");
        std::fs::write(store.dir().join("README.txt"), "not an entry\n").expect("wrote");

        let shelf = store.load();
        assert_eq!(shelf.ids(), ["abcd1234"], "the good entry is still there");
        let named: Vec<&str> = shelf.unreadable.iter().map(|u| u.file.as_str()).collect();
        assert_eq!(
            named,
            ["broken.json", "cron.json"],
            "the lock and the readme are not entries; a broken entry is"
        );
        assert!(
            shelf.unreadable[1].why.contains("every"),
            "the reason names the grammar: {:?}",
            shelf.unreadable[1]
        );
    }
}
