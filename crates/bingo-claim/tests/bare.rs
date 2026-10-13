use bingo_claim::lock;

#[test]
fn a_bare_lock_excludes_until_drop_and_preserves_the_file() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join(".lock");
    let first = lock(&path).unwrap().unwrap();
    assert!(lock(&path).unwrap().is_none());
    assert!(std::fs::read(&path).unwrap().is_empty());
    drop(first);
    assert!(path.exists());
    assert!(lock(&path).unwrap().is_some());
    std::fs::write(&path, "preserved").unwrap();
    let _held = lock(&path).unwrap().unwrap();
    assert_eq!(std::fs::read_to_string(path).unwrap(), "preserved");
}
