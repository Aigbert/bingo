use bingo_claim::lock;

#[test]
fn a_bare_lock_excludes_until_drop_and_preserves_the_file() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join(".lock");
    // Windows locks are mandatory: another handle reads the file only once
    // the lock is let go.
    let first = lock(&path).unwrap().unwrap();
    assert!(lock(&path).unwrap().is_none());
    drop(first);
    assert!(std::fs::read(&path).unwrap().is_empty());
    assert!(lock(&path).unwrap().is_some());
    std::fs::write(&path, "preserved").unwrap();
    drop(lock(&path).unwrap().unwrap());
    assert_eq!(std::fs::read_to_string(path).unwrap(), "preserved");
}
