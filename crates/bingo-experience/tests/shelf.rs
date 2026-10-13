use bingo_experience::Library;

#[test]
fn reading_hand_edited_experience_never_rewrites_it() {
    let dir = tempfile::tempdir().unwrap();
    let library = Library::new(dir.path());
    let cwd = dir.path();
    let path = library.path(cwd, "mine");
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    let text = "---\n# Keep my comments\nsummary: A hand-edited entry\ncustom: preserved\ncreated: 2026-10-13T00:00:00Z\n---\n\nMy notes.\n";
    std::fs::write(&path, text).unwrap();
    let loaded = library.load(cwd);
    assert!(loaded.unreadable.is_empty());
    assert_eq!(loaded.entries[0].id, "mine");
    assert_eq!(std::fs::read_to_string(path).unwrap(), text);
}
