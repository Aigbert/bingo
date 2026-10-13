use bingo_shelf::diff::unified;
use std::path::Path;

#[test]
fn a_new_file_is_all_additions_under_its_name() {
    let unified = unified(
        Path::new("/store/ab12.json"),
        "",
        "{\n  \"spec\": \"every 30m\"\n}\n",
    );
    assert!(unified.starts_with("--- /store/ab12.json\n+++ /store/ab12.json\n"));
    assert!(unified.contains("+  \"spec\": \"every 30m\""), "{unified}");
}

#[test]
fn nothing_changed_is_no_diff_at_all() {
    assert_eq!(unified(Path::new("a"), "same\n", "same\n"), "");
}

#[test]
fn a_changed_line_is_a_hunk_under_the_file_name() {
    let unified = unified(Path::new("/work/a.txt"), "one\ntwo\n", "one\ntoo\n");
    assert!(unified.starts_with("--- /work/a.txt\n+++ /work/a.txt\n"));
    assert!(unified.contains("-two\n"), "got {unified}");
    assert!(unified.contains("+too\n"), "got {unified}");
}
