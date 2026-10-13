use bingo_schedule::Store;

#[test]
fn pre_m129_schedule_bytes_survive_loading_and_saving() {
    let dir = tempfile::tempdir().unwrap();
    let store = Store::new(dir.path());
    std::fs::create_dir_all(store.dir()).unwrap();
    let path = store.path("old");
    let bytes = include_bytes!("../fixtures/pre-m129.json");
    std::fs::write(&path, bytes).unwrap();
    let loaded = store.load();
    assert!(loaded.unreadable.is_empty());
    assert_eq!(loaded.entries[0].id, "old");
    store.save(&loaded.entries[0]).unwrap();
    assert_eq!(std::fs::read(path).unwrap(), bytes);
}
