#![allow(clippy::unwrap_used, clippy::expect_used)]

use bingo_sdk::{Event, Frame, Item, ItemBody};

#[test]
fn unknown_and_malformed_events_keep_their_bytes() {
    for wire in [
        include_str!("../fixtures/record/unknown/event.json"),
        include_str!("../fixtures/record/unknown/malformed-event.json"),
    ] {
        let raw = wire.trim_end();
        let event: Event = serde_json::from_str(raw).expect("opaque event");
        assert!(matches!(event, Event::Unknown { .. }));
        assert!(event.is_durable());
        assert_eq!(serde_json::to_string(&event).unwrap(), raw);
        let frame: Frame = serde_json::from_str(&format!(
            r#"{{"seq":7,"ts":"2023-11-14T22:13:20Z","session":"ses_1","event":{raw}}}"#
        ))
        .unwrap();
        assert_eq!(frame.seq.0, 7);
        assert_eq!(serde_json::to_string(&frame.event).unwrap(), raw);
    }
}

#[test]
fn unknown_and_malformed_items_keep_their_envelope_and_bytes() {
    for wire in [
        include_str!("../fixtures/record/unknown/item.json"),
        include_str!("../fixtures/record/unknown/malformed-item.json"),
    ] {
        let raw = wire.trim_end();
        let body: ItemBody = serde_json::from_str(raw).expect("opaque item body");
        assert!(matches!(body, ItemBody::Unknown { .. }));
        assert_eq!(serde_json::to_string(&body).unwrap(), raw);
        let item_wire = format!(
            r#"{{"id":"itm_1","turn":"trn_1","round":0,"status":"completed","startedAt":"2023-11-14T22:13:20Z","body":{raw}}}"#
        );
        let item: Item = serde_json::from_str(&item_wire).unwrap();
        assert_eq!(item.id.as_str(), "itm_1");
        assert_eq!(item.turn.as_ref().unwrap().as_str(), "trn_1");
        assert!(item.is_terminal());
        assert_eq!(serde_json::to_string(&item).unwrap(), item_wire);
        let event: Event =
            serde_json::from_str(&format!(r#"{{"type":"itemCompleted","item":{item_wire}}}"#))
                .unwrap();
        let Event::ItemCompleted { item } = event else {
            panic!("item envelope must stay known")
        };
        assert_eq!(serde_json::to_string(&item.body).unwrap(), raw);
    }
}

#[test]
fn unknown_events_advance_sequence_without_changing_the_view() {
    let Event::SessionUpdated { summary } =
        serde_json::from_str(include_str!("../fixtures/record/event/sessionUpdated.json")).unwrap()
    else {
        panic!("head")
    };
    let mut state = bingo_sdk::SessionState::new(summary);
    let frame: Frame = serde_json::from_str(
        r#"{"seq":7,"ts":"2023-11-14T22:13:20Z","session":"ses_1","event":{"type":"futureEvent"}}"#,
    )
    .unwrap();
    let before = state.clone();
    assert_eq!(state.apply(&frame), bingo_sdk::Applied::Nothing);
    assert_eq!(state.seq, frame.seq);
    state.seq = before.seq;
    assert_eq!(state, before);
}

#[test]
fn the_transcript_retains_opaque_items_through_snapshots_and_updates() {
    let mut item: Item = serde_json::from_str(r#"{"id":"itm_1","turn":"trn_1","status":"running","startedAt":"2023-11-14T22:13:20Z","body":{"kind":"futureItem","data":42}}"#).unwrap();
    let mut transcript = bingo_sdk::Transcript::new();
    transcript.apply(&Event::ItemStarted { item: item.clone() });
    transcript.apply(&Event::ItemDelta {
        item: item.id.clone(),
        n: 1,
        kind: bingo_sdk::DeltaKind::Text,
        data: "ignored".into(),
    });
    item.status = bingo_sdk::ItemStatus::Completed;
    transcript.apply(&Event::ItemCompleted { item: item.clone() });
    assert_eq!(transcript.items().len(), 1);
    assert_eq!(transcript.get(&item.id), Some(&item));
    let restored: bingo_sdk::Transcript =
        serde_json::from_str(&serde_json::to_string(&transcript).unwrap()).unwrap();
    assert_eq!(restored, transcript);
}

#[test]
fn invalid_envelopes_are_not_opaque_payloads() {
    for wire in [
        r#"{"seq":"bad","ts":"2023-11-14T22:13:20Z","session":"ses_1","event":{"type":"futureEvent"}}"#,
        r#"{"seq":1,"ts":"not-a-timestamp","session":"ses_1","event":{"type":"futureEvent"}}"#,
        r#"{"seq":1,"ts":"2023-11-14T22:13:20Z","session":"ses_1"}"#,
    ] {
        assert!(serde_json::from_str::<Frame>(wire).is_err());
    }
    assert!(serde_json::from_str::<Item>(r#"{"id":"itm_1","status":"bad","startedAt":"2023-11-14T22:13:20Z","body":{"kind":"futureItem"}}"#).is_err());
}

#[test]
fn buffered_payloads_and_values_keep_the_unknown_data() {
    #[derive(serde::Deserialize)]
    #[serde(tag = "kind")]
    enum Wrapper {
        Frame { frame: Frame },
    }
    let wire = r#"{"seq":1,"ts":"2023-11-14T22:13:20Z","session":"ses_1","event":{"type":"futureEvent","data":42}}"#;
    let value: serde_json::Value = serde_json::from_str(wire).unwrap();
    let frame: Frame = serde_json::from_value(value.clone()).unwrap();
    assert_eq!(serde_json::to_value(&frame).unwrap(), value);
    let Wrapper::Frame { frame } =
        serde_json::from_str(&format!(r#"{{"kind":"Frame","frame":{wire}}}"#)).unwrap();
    assert_eq!(serde_json::to_value(&frame).unwrap(), value);
}
