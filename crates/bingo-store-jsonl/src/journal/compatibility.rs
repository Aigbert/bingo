use super::*;
use bingo_sdk::{Event, InteractionKind, ItemBody, View};

fn replay_fixture(name: &str) -> Vec<Frame> {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(layout::journal(dir.path()), crate::tests::fixture(name)).unwrap();
    replay(dir.path(), Seq::ZERO).expect("compatible journal")
}

#[test]
fn historical_shapes_stay_known() {
    let frames = replay_fixture("historical.jsonl");
    assert_eq!(frames.len(), 6);
    assert!(matches!(&frames[1].event, Event::ItemCompleted { item }
        if matches!(&item.body, ItemBody::ToolCall { .. })));
    assert!(matches!(&frames[2].event, Event::ItemCompleted { item }
        if matches!(&item.body, ItemBody::ToolCall { output: Some(output), .. }
            if matches!(&output.display, Some(View::Custom { kind, .. }) if kind == "summary"))));
    assert!(
        matches!(&frames[3].event, Event::InteractionOpened { interaction }
        if matches!(&interaction.kind, InteractionKind::Question(_)))
    );
}

#[test]
fn an_unknown_event_journal_replays() {
    let frames = replay_fixture("unknown-event.jsonl");
    assert_eq!(frames.len(), 4);
    assert!(matches!(frames[1].event, Event::Unknown { .. }));
}

#[test]
fn an_unknown_item_journal_replays() {
    let frames = replay_fixture("unknown-item.jsonl");
    assert_eq!(frames.len(), 4);
    let Event::ItemCompleted { item } = &frames[1].event else {
        panic!("known envelope")
    };
    assert!(matches!(item.body, ItemBody::Unknown { .. }));
}

#[test]
fn replay_and_append_preserve_unknown_payload_bytes() {
    let dir = tempfile::tempdir().unwrap();
    create(dir.path(), &crate::tests::session()).unwrap();
    for (index, raw) in [
        include_str!("../../../bingo-sdk/fixtures/record/unknown/event.json"),
        include_str!("../../../bingo-sdk/fixtures/record/unknown/malformed-event.json"),
    ]
    .iter()
    .enumerate()
    {
        let event: Event = serde_json::from_str(raw).unwrap();
        append(dir.path(), &crate::tests::frame(index as u64 + 1, event)).unwrap();
    }
    append_version(dir.path()).unwrap();
    let frames = replay(dir.path(), Seq::ZERO).unwrap();
    let raw = include_str!("../../../bingo-sdk/fixtures/record/unknown/event.json").trim_end();
    assert_eq!(serde_json::to_string(&frames[0].event).unwrap(), raw);
    assert!(
        std::fs::read_to_string(layout::journal(dir.path()))
            .unwrap()
            .contains(raw)
    );
}

#[test]
fn version_one_opaque_payloads_keep_their_bytes_across_the_identity_step() {
    let dir = tempfile::tempdir().unwrap();
    let raw = r#"{ "type" : "future", "parts" : [{ "type":"image", "data":"eA==" }] }"#;
    let event: Event = serde_json::from_str(raw).unwrap();
    let frame = crate::tests::frame(1, event);
    let header = serde_json::json!({"format": FORMAT, "version": 1, "session": frame.session});
    let line = serde_json::to_string(&frame).unwrap();
    std::fs::write(layout::journal(dir.path()), format!("{header}\n{line}\n")).unwrap();
    let frames = replay(dir.path(), Seq::ZERO).unwrap();
    assert_eq!(serde_json::to_string(&frames[0].event).unwrap(), raw);
    append(dir.path(), &crate::tests::frame(2, frames[0].event.clone())).unwrap();
    assert!(!dir.path().join("blobs").exists());
    assert_eq!(serde_json::to_string(&replay(dir.path(), Seq::ZERO).unwrap()[1].event).unwrap(), raw);
}
