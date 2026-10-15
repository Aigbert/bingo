use super::*;
fn fixture() -> tempfile::TempDir {
    let home = tempfile::tempdir().unwrap();
    std::fs::write(home.path().join("rule.md"), "fact").unwrap();
    std::fs::write(
        home.path().join("MEMORY.md"),
        "# Memory\r\n[rule](rule.md)\r\n\r\nkeep\n",
    )
    .unwrap();
    home
}
#[test]
fn replacement_is_atomic_and_preserves_other_bytes() {
    let home = fixture();
    let index = home.path().join("MEMORY.md");
    let previous = home.path().join("previous");
    std::fs::hard_link(&index, &previous).unwrap();
    let original = std::fs::read_to_string(&previous).unwrap();
    forget(home.path(), "rule").unwrap();
    assert_eq!(
        std::fs::read_to_string(previous).unwrap(),
        original,
        "in-place writing mutates the old inode"
    );
    assert_eq!(
        std::fs::read_to_string(index).unwrap(),
        "# Memory\r\n\r\nkeep\n"
    );
    assert_eq!(std::fs::read_dir(home.path()).unwrap().count(), 2);
    assert_eq!(
        forget(home.path(), "rule").unwrap_err().code,
        ErrorCode::NotFound
    );
}
#[test]
fn a_temp_collision_leaves_the_file_and_index_intact() {
    let home = fixture();
    let collision = home.path().join("occupied.tmp");
    std::fs::write(&collision, "owned by someone else").unwrap();
    let index = std::fs::read_to_string(home.path().join("MEMORY.md")).unwrap();
    assert!(
        forget_with(
            home.path(),
            "rule",
            |_| collision.clone(),
            |_| {},
            sync_directory
        )
        .is_err()
    );
    assert!(home.path().join("rule.md").exists());
    assert_eq!(
        std::fs::read_to_string(home.path().join("MEMORY.md")).unwrap(),
        index
    );
    assert_eq!(
        std::fs::read_to_string(collision).unwrap(),
        "owned by someone else"
    );
}
#[test]
fn one_concurrent_edit_is_retried_without_losing_it() {
    let home = fixture();
    forget_with(
        home.path(),
        "rule",
        temporary,
        |attempt| {
            if attempt == 0 {
                std::fs::write(home.path().join("MEMORY.md"), "[rule](rule.md)\nnew fact\n")
                    .unwrap();
            }
        },
        sync_directory,
    )
    .unwrap();
    assert_eq!(
        std::fs::read_to_string(home.path().join("MEMORY.md")).unwrap(),
        "new fact\n"
    );
}
#[test]
fn two_concurrent_edits_refuse_to_forget() {
    let home = fixture();
    let error = forget_with(
        home.path(),
        "rule",
        temporary,
        |attempt| {
            std::fs::write(
                home.path().join("MEMORY.md"),
                format!("[rule](rule.md)\nedit {attempt}\n"),
            )
            .unwrap();
        },
        sync_directory,
    )
    .unwrap_err();
    assert!(error.message.contains("changed twice"));
    assert!(home.path().join("rule.md").exists());
    assert_eq!(std::fs::read_dir(home.path()).unwrap().count(), 2);
}
#[test]
fn unlink_failure_keeps_the_safe_index_without_the_link() {
    let home = fixture();
    std::fs::remove_file(home.path().join("rule.md")).unwrap();
    std::fs::create_dir(home.path().join("rule.md")).unwrap();
    assert!(forget(home.path(), "rule").is_err());
    assert!(
        !std::fs::read_to_string(home.path().join("MEMORY.md"))
            .unwrap()
            .contains("rule.md")
    );
}
#[test]
fn invalid_names_do_not_touch_the_store() {
    let home = fixture();
    for name in ["../rule", "MEMORY", "a/b", "a\\b", "", "a:", "rule."] {
        assert_eq!(
            forget(home.path(), name).unwrap_err().code,
            ErrorCode::InvalidInput
        );
    }
    assert!(home.path().join("rule.md").exists());
}
#[test]
fn a_failed_directory_sync_still_removes_the_file() {
    let home = fixture();
    forget_with(
        home.path(),
        "rule",
        temporary,
        |_| {},
        |_| Err(std::io::Error::other("sync refused")),
    )
    .unwrap();
    assert!(!home.path().join("rule.md").exists());
    assert_eq!(
        std::fs::read_to_string(home.path().join("MEMORY.md")).unwrap(),
        "# Memory\r\n\r\nkeep\n"
    );
}
