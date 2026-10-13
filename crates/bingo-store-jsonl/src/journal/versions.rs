use super::*;
use bingo_sdk::ErrorCode;

fn planted(lines: &[String]) -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(layout::journal(dir.path()), lines.join("\n") + "\n").unwrap();
    dir
}

fn clean() -> Vec<String> {
    crate::tests::fixture("clean.jsonl")
        .lines()
        .map(str::to_owned)
        .collect()
}

#[test]
fn version_lines_between_frames_do_not_take_a_sequence_number() {
    let mut lines = clean();
    lines.insert(2, r#"{"format":"bingo-journal","version":1}"#.into());
    lines.push(r#"{"format":"bingo-journal","version":1}"#.into());
    let dir = planted(&lines);
    let frames = replay(dir.path(), Seq::ZERO).unwrap();
    assert_eq!(frames.iter().map(|f| f.seq.0).collect::<Vec<_>>(), [1, 2]);
    assert_eq!(replay(dir.path(), Seq(1)).unwrap().len(), 1);
}

#[test]
fn a_newer_version_line_is_refused_even_at_the_tail_or_before_since() {
    for tail in [false, true] {
        let mut lines = clean();
        let at = if tail { lines.len() } else { 2 };
        lines.insert(at, r#"{"format":"bingo-journal","version":2}"#.into());
        let dir = planted(&lines);
        let error = replay(dir.path(), Seq(99)).expect_err("newer format must be refused");
        assert_eq!(error.code, ErrorCode::Storage);
        assert!(error.message.contains("journal version 2"), "{error}");
        assert!(error.message.contains("this build reads 1"), "{error}");
    }
}

#[test]
fn a_malformed_version_line_in_the_middle_is_corruption() {
    let mut lines = clean();
    lines.insert(2, r#"{"format":"bingo-journal","version":"one"}"#.into());
    let dir = planted(&lines);
    let error = replay(dir.path(), Seq::ZERO).unwrap_err();
    assert_eq!(error.code, ErrorCode::Storage);
    assert!(error.message.contains("line 3"), "{error}");
}

#[test]
fn the_version_writer_only_appends_the_marker() {
    let lines = clean();
    let dir = planted(&lines);
    let before = std::fs::read_to_string(layout::journal(dir.path())).unwrap();
    append_version(dir.path()).unwrap();
    let after = std::fs::read_to_string(layout::journal(dir.path())).unwrap();
    assert_eq!(
        after,
        before + "{\"format\":\"bingo-journal\",\"version\":1}\n"
    );
    assert_eq!(replay(dir.path(), Seq::ZERO).unwrap().len(), 2);
}
