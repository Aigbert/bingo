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
    assert!(format!("{:?}", frames[1].event).starts_with("Unknown"));
}

#[test]
fn an_unknown_item_journal_replays() {
    let frames = replay_fixture("unknown-item.jsonl");
    assert_eq!(frames.len(), 4);
    let Event::ItemCompleted { item } = &frames[1].event else {
        panic!("known envelope")
    };
    assert!(format!("{:?}", item.body).starts_with("Unknown"));
}
