use super::*;
use bingo_sdk::{ContentPart, Event, ItemBody};

fn picture(frame: &Frame) -> &[ContentPart] {
    let Event::ItemCompleted { item } = &frame.event else {
        panic!("item")
    };
    let ItemBody::User { parts, .. } = &item.body else {
        panic!("user")
    };
    parts
}

fn planted(version: u32) -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    let fixture = match version {
        1 => include_str!("../../fixtures/picture-v1.jsonl"),
        _ => include_str!("../../fixtures/picture-v2.jsonl"),
    };
    std::fs::write(layout::journal(dir.path()), fixture).unwrap();
    dir
}

#[test]
fn inline_and_blob_fixtures_resolve_to_the_same_image() {
    let old = planted(1);
    let new = planted(2);
    let hash = bingo_digest::hex(&bingo_digest::digest(b"x"));
    std::fs::create_dir(new.path().join("blobs")).unwrap();
    std::fs::write(new.path().join("blobs").join(hash), b"x").unwrap();
    assert_eq!(
        replay(old.path(), Seq::ZERO).unwrap(),
        replay(new.path(), Seq::ZERO).unwrap()
    );
}

#[test]
fn two_picture_appends_keep_one_blob_and_upgrade_the_journal() {
    let dir = planted(1);
    let mut frame = replay(dir.path(), Seq::ZERO).unwrap().remove(0);
    frame.seq = Seq(2);
    append(dir.path(), &frame).unwrap();
    frame.seq = Seq(3);
    append(dir.path(), &frame).unwrap();
    assert_eq!(
        std::fs::read_dir(dir.path().join("blobs")).unwrap().count(),
        1
    );
    let text = std::fs::read_to_string(layout::journal(dir.path())).unwrap();
    assert_eq!(text.matches("\"blob\"").count(), 2);
    assert_eq!(text.matches("\"version\":2").count(), 1);
    assert_eq!(replay(dir.path(), Seq::ZERO).unwrap().len(), 3);
}

#[test]
fn a_missing_blob_becomes_a_text_part() {
    let dir = planted(2);
    let frames = replay(dir.path(), Seq::ZERO).unwrap();
    assert_eq!(
        picture(&frames[0]),
        [ContentPart::text("[picture missing: image/png 0.0 KB]")]
    );
}

fn realistic_picture_record() -> serde_json::Value {
    let line = include_str!("../../fixtures/picture-v2.jsonl")
        .lines()
        .nth(1)
        .unwrap();
    let mut record: serde_json::Value = serde_json::from_str(line).unwrap();
    record["session"] = SessionId::mint().to_string().into();
    record["event"]["item"]["id"] = bingo_sdk::ItemId::mint().to_string().into();
    record
}

fn picture_record_line(mut record: serde_json::Value, bytes: &[u8]) -> String {
    let image = &mut record["event"]["item"]["body"]["parts"][0];
    image["blob"] = bingo_digest::hex(&bingo_digest::digest(bytes)).into();
    image["bytes"] = bytes.len().into();
    serde_json::to_string(&record).unwrap()
}

#[test]
fn the_planned_v2_picture_line_is_under_one_kib() {
    let line = picture_record_line(realistic_picture_record(), &vec![0; 3_000_000]);
    assert!(line.len() < 1024, "{} bytes before newline", line.len());
}

#[test]
fn the_planned_v2_picture_line_length_is_independent_of_picture_size() {
    let record = realistic_picture_record();
    let small = picture_record_line(record.clone(), b"x");
    let large = picture_record_line(record, &vec![0; 3_000_000]);
    assert_eq!(
        large.len() - small.len(),
        3_000_000usize.to_string().len() - 1usize.to_string().len()
    );
}
