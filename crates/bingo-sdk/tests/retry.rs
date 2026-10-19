#![allow(clippy::unwrap_used, clippy::expect_used)]
use bingo_sdk::{Event, Frame, SessionState};
use serde_json::json;

fn fold(retry: serde_json::Value) -> SessionState {
    fold_all(vec![retry])
}

fn fold_all(events: Vec<serde_json::Value>) -> SessionState {
    let Event::SessionUpdated { summary } =
        serde_json::from_str(include_str!("../fixtures/record/event/sessionUpdated.json")).unwrap()
    else {
        panic!("summary")
    };
    let mut state = SessionState::new(summary);
    let started = json!({"type":"turnStarted","turn":"trn_1","inputs":[],"origin":"submit"});
    for (seq, event) in std::iter::once(started).chain(events).enumerate() {
        let frame: Frame = serde_json::from_value(json!({"session":state.summary.id,"seq":seq + 1,"ts":"2026-01-01T00:00:00Z","event":event})).unwrap();
        state.apply(&frame);
    }
    state
}

#[test]
fn retry_fold_retains_the_wait_and_its_frame_clock() {
    let event =
        serde_json::from_str(include_str!("../fixtures/record/event/turnRetrying.json")).unwrap();
    let state = fold(event);
    let retry = state.turn.unwrap().retrying.unwrap();
    assert_eq!(retry.delay_ms, 500);
    assert_eq!(retry.reason, "server error 503");
    assert_eq!(retry.started_at.to_string(), "2026-01-01T00:00:00Z");
}

#[test]
fn an_old_retry_journal_and_snapshot_still_fold() {
    let state =
        fold(json!({"type":"turnRetrying","turn":"trn_1","attempt":2,"max":10,"dropped":[]}));
    let retry = state.turn.unwrap().retrying.unwrap();
    assert_eq!(retry.delay_ms, 0);
    assert_eq!(retry.reason, "");
    let retry: bingo_sdk::Retry = serde_json::from_value(json!({"attempt":2,"max":10})).unwrap();
    assert_eq!(retry.delay_ms, 0);
    assert_eq!(retry.reason, "");
}

#[test]
fn spinner_exposes_retry_facts_without_a_cached_countdown() {
    let wire = json!({"component":"Spinner","props":{
        "word":"rate limited","message":null,"suffix":"","mode":"retry",
        "retry":{"attempt":2,"max":6,"delayMs":3000,"reason":"rate limited","startedAt":"2026-01-01T00:00:00Z"}
    }});
    let props: bingo_sdk::component::Props = serde_json::from_value(wire.clone()).unwrap();
    assert_eq!(serde_json::to_value(props).unwrap(), wire);
}

#[test]
fn receipt_facts_round_trip_without_session_bodies() {
    let wire = json!({"id":"ses_1","title":"hello","turns":2,"durationMs":4900,"editedFiles":1});
    let facts: bingo_sdk::component::ReceiptFacts = serde_json::from_value(wire.clone()).unwrap();
    assert_eq!(serde_json::to_value(facts).unwrap(), wire);
}

#[test]
fn retry_leaves_on_the_live_turns_first_provider_item() {
    let mut state =
        fold(json!({"type":"turnRetrying","turn":"trn_1","attempt":2,"max":10,"dropped":[]}));
    let mut item: bingo_sdk::Item =
        serde_json::from_str(include_str!("../fixtures/headline/item.json")).unwrap();
    item.turn = Some(bingo_sdk::TurnId::from_raw("unrelated"));
    let mut frame = Frame {
        session: state.summary.id.clone(),
        seq: bingo_sdk::Seq(3),
        cause: None,
        ts: jiff::Timestamp::UNIX_EPOCH,
        event: Event::ItemStarted { item: item.clone() },
    };
    state.apply(&frame);
    assert!(state.turn.as_ref().unwrap().retrying.is_some());
    item.turn = Some(bingo_sdk::TurnId::from_raw("trn_1"));
    item.body = bingo_sdk::ItemBody::Assistant {
        text: String::new(),
    };
    frame.seq = bingo_sdk::Seq(4);
    frame.event = Event::ItemStarted { item };
    state.apply(&frame);
    assert!(state.turn.as_ref().unwrap().retrying.is_none());
}

#[test]
fn deltas_on_an_item_from_before_the_retry_keep_the_row() {
    let started: serde_json::Value =
        serde_json::from_str(include_str!("../fixtures/record/event/itemStarted.json")).unwrap();
    let delta: serde_json::Value =
        serde_json::from_str(include_str!("../fixtures/record/event/itemDelta.json")).unwrap();
    let state = fold_all(vec![
        started,
        json!({"type":"turnRetrying","turn":"trn_1","attempt":2,"max":10,"dropped":[]}),
        delta,
    ]);
    assert!(state.turn.unwrap().retrying.is_some());
}
