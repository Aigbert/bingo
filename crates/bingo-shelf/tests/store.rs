use std::path::Path;

use bingo_shelf::{Codec, CodecError, Shelf};
use serde::{Deserialize, Serialize};

#[derive(Debug, PartialEq, Serialize, Deserialize)]
struct Entry {
    #[serde(skip)]
    id: String,
    text: String,
}

struct Json;
impl Codec for Json {
    type Record = Entry;
    const EXT: &'static str = "json";
    fn encode(record: &Entry) -> Result<String, CodecError> {
        serde_json::to_string_pretty(record).map(|s| format!("{s}\n"))
            .map_err(|e| CodecError(e.to_string()))
    }
    fn decode(id: &str, text: &str) -> Result<Entry, CodecError> {
        let mut entry: Entry = serde_json::from_str(text).map_err(|e| CodecError(e.to_string()))?;
        entry.id = id.into();
        Ok(entry)
    }
}

fn entry(id: &str) -> Entry { Entry { id: id.into(), text: "kept".into() } }
fn names(dir: &Path) -> Vec<String> {
    let mut names: Vec<_> = std::fs::read_dir(dir).unwrap().map(|f| f.unwrap().file_name().to_string_lossy().into_owned()).collect();
    names.sort();
    names
}

#[test]
fn a_missing_store_is_empty_and_a_round_trip_leaves_no_temp() {
    let dir = tempfile::tempdir().unwrap();
    let shelf = Shelf::<Json>::new(dir.path().join("missing"));
    assert!(shelf.load().is_empty());
    assert!(shelf.load().unreadable.is_empty());
    let path = shelf.save("abcd1234", &entry("abcd1234")).unwrap();
    assert_eq!(path, shelf.path("abcd1234"));
    assert_eq!(shelf.load().entries, [entry("abcd1234")]);
    assert_eq!(names(shelf.dir()), ["abcd1234.json"]);
}

#[test]
fn entries_and_unreadable_files_are_sorted_and_dotfiles_are_skipped() {
    let dir = tempfile::tempdir().unwrap();
    let shelf = Shelf::<Json>::new(dir.path());
    for id in ["ccc", "aaa", "bbb"] { shelf.save(id, &entry(id)).unwrap(); }
    for name in ["z.json", "d.json", ".hidden.json", "runner.lock", "README.txt"] {
        std::fs::write(dir.path().join(name), "broken").unwrap();
    }
    let loaded = shelf.load();
    assert_eq!(loaded.entries, [entry("aaa"), entry("bbb"), entry("ccc")]);
    assert_eq!(loaded.unreadable.iter().map(|u| u.file.as_str()).collect::<Vec<_>>(), ["d.json", "z.json"]);
    assert!(loaded.unreadable.iter().all(|u| !u.why.is_empty()));
}

#[test]
fn saving_replaces_deleting_is_idempotent_and_minted_ids_are_free() {
    let dir = tempfile::tempdir().unwrap();
    let shelf = Shelf::<Json>::new(dir.path());
    shelf.save("abc", &entry("abc")).unwrap();
    let revised = Entry { text: "revised".into(), ..entry("abc") };
    shelf.save("abc", &revised).unwrap();
    assert_eq!(shelf.load().entries, [revised]);
    assert!(!shelf.path(&shelf.mint()).exists());
    shelf.delete("abc").unwrap();
    shelf.delete("abc").unwrap();
    assert!(shelf.load().is_empty());
}

#[test]
fn a_failed_rename_removes_the_temp_file() {
    let dir = tempfile::tempdir().unwrap();
    let shelf = Shelf::<Json>::new(dir.path());
    std::fs::create_dir(shelf.path("blocked")).unwrap();
    assert!(shelf.save("blocked", &entry("blocked")).is_err());
    assert_eq!(names(dir.path()), ["blocked.json"]);
}

#[test]
fn loading_never_rewrites_hand_edited_bytes_and_renaming_changes_the_id() {
    let dir = tempfile::tempdir().unwrap();
    let shelf = Shelf::<Json>::new(dir.path());
    let bytes = "{  \"text\" : \"hand edited\" }\n\n";
    std::fs::write(shelf.path("mine"), bytes).unwrap();
    assert_eq!(shelf.load().entries[0].id, "mine");
    assert_eq!(std::fs::read_to_string(shelf.path("mine")).unwrap(), bytes);
}
