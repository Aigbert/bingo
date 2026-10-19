#![allow(clippy::unwrap_used, clippy::expect_used)]
use bingo_sdk::{
    Event, Item, ItemBody, ItemStatus,
    transcript::{LiveCall, call_waiting, live_call},
};
use jiff::Timestamp;

fn call() -> Item {
    serde_json::from_value(serde_json::json!({"id":"call","status":"pending",
        "startedAt":"2026-01-01T00:00:00Z","body":{"kind":"toolCall","callId":"c","name":"Edit","input":{}}})).unwrap()
}
#[test]
fn only_unsettled_calls_can_be_asked_or_running() {
    let mut item = call();
    let Event::InteractionOpened {
        interaction: mut ask,
    } = serde_json::from_str(include_str!(
        "../fixtures/record/event/interactionOpened.json"
    ))
    .unwrap()
    else {
        panic!("ask")
    };
    ask.item = Some(item.id.clone());
    use ItemStatus::{Completed, Failed, Interrupted, Pending, Running};
    use LiveCall::{Asked, Running as Runs};
    let cases = [
        (Pending, false, false, None),
        (Pending, false, true, Some(Asked)),
        (Pending, true, false, Some(Asked)),
        (Pending, true, true, Some(Asked)),
        (Running, false, false, Some(Runs)),
        (Running, false, true, Some(Asked)),
        (Running, true, false, Some(Asked)),
        (Running, true, true, Some(Asked)),
        (Completed, false, false, None),
        (Completed, false, true, None),
        (Completed, true, false, None),
        (Completed, true, true, None),
        (Failed, false, false, None),
        (Failed, false, true, None),
        (Failed, true, false, None),
        (Failed, true, true, None),
        (Interrupted, false, false, None),
        (Interrupted, false, true, None),
        (Interrupted, true, false, None),
        (Interrupted, true, true, None),
    ];
    for (status, waiting, open, expected) in cases {
        item.status = status;
        let ItemBody::ToolCall { asked_at, .. } = &mut item.body else {
            panic!("call")
        };
        *asked_at = waiting.then_some(Timestamp::UNIX_EPOCH);
        let asks = if open {
            std::slice::from_ref(&ask)
        } else {
            &[]
        };
        assert_eq!(
            live_call(&item, asks),
            expected,
            "{status:?}, waiting={waiting}, open={open}"
        );
    }
    let mut text = item.clone();
    text.body = ItemBody::Assistant {
        text: "still writing".into(),
    };
    text.status = ItemStatus::Running;
    assert_eq!(live_call(&text, &[]), None);
}
#[test]
fn a_call_wait_closes_when_it_begins_or_completes() {
    let mut item = call();
    assert!(!call_waiting(&item));
    let ItemBody::ToolCall { asked_at, .. } = &mut item.body else {
        panic!("call")
    };
    *asked_at = Some(Timestamp::UNIX_EPOCH);
    assert!(call_waiting(&item));
    item.completed_at = Some(Timestamp::UNIX_EPOCH);
    assert!(!call_waiting(&item));
    item.completed_at = None;
    let ItemBody::ToolCall { began_at, .. } = &mut item.body else {
        panic!("call")
    };
    *began_at = Some(Timestamp::UNIX_EPOCH);
    assert!(!call_waiting(&item));
}
