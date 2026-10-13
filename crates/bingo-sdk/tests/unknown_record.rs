use bingo_sdk::{Event, Frame, Item, ItemBody};

#[test]
fn unknown_and_malformed_events_keep_their_bytes() {
    for wire in [
        include_str!("../fixtures/record/unknown/event.json"),
        include_str!("../fixtures/record/unknown/malformed-event.json"),
    ] {
        let raw = wire.trim_end();
        let event: Event = serde_json::from_str(raw).expect("opaque event");
        assert!(format!("{event:?}").starts_with("Unknown"));
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
        assert!(format!("{body:?}").starts_with("Unknown"));
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
