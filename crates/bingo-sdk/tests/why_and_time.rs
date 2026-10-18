#![allow(clippy::unwrap_used, clippy::expect_used)]

use bingo_sdk::{Event, Frame, ItemBody, Reason, SessionState};
use serde_json::{Value, json};

#[test]
fn optional_provenance_and_ask_times_round_trip() {
    for body in [
        json!({"kind":"toolCall","callId":"c","name":"Edit","input":{},
            "askedAt":"2026-10-18T00:00:01Z","beganAt":"2026-10-18T00:00:02Z"}),
        json!({"kind":"interruption","marker":"stopped","askedAt":"2026-10-18T00:00:01Z"}),
        json!({"kind":"toolCall","callId":"c","name":"Edit","input":{},
            "allowedBy":{"kind":"mode","mode":"acceptEdits"}}),
        json!({"kind":"toolCall","callId":"c","name":"Bash","input":{},
            "allowedBy":{"kind":"rule","rule":"Bash(ls)","source":"session"}}),
        json!({"kind":"toolCall","callId":"c","name":"Bash","input":{},
            "allowedBy":{"kind":"rule","rule":"Bash(ls)","source":"settings"}}),
        json!({"kind":"toolCall","callId":"c","name":"Bash","input":{},
            "allowedBy":{"kind":"hook","hook":"review"}}),
        json!({"kind":"permissionReceipt","interaction":"q","tool":"Edit",
            "decision":"allow","askedAt":"2026-10-18T00:00:01Z"}),
        json!({"kind":"questionAnswer","interaction":"q","question":"Where?",
            "answer":"Here","askedAt":"2026-10-18T00:00:01Z"}),
    ] {
        let decoded: ItemBody = serde_json::from_value(body.clone()).unwrap();
        assert!(!matches!(decoded, ItemBody::Unknown { .. }));
        assert_eq!(serde_json::to_value(decoded).unwrap(), body);
        replay(body);
    }
}

fn replay(body: Value) {
    let wire = json!({"seq":1,"ts":"2026-10-18T00:00:03Z","session":"s",
        "event":{"type":"itemCompleted","item":{"id":"i","turn":"t","round":0,
        "status":"completed","startedAt":"2026-10-18T00:00:03Z","body":body}}});
    let frame: Frame = serde_json::from_value(wire.clone()).unwrap();
    assert!(matches!(frame.event, Event::ItemCompleted { .. }));
    assert_eq!(serde_json::to_value(&frame).unwrap(), wire);
    let Event::SessionUpdated { summary } =
        serde_json::from_str(include_str!("../fixtures/record/event/sessionUpdated.json")).unwrap()
    else {
        panic!("summary")
    };
    let mut state = SessionState::new(summary);
    state.apply(&frame);
    assert_eq!(
        serde_json::to_value(&state.transcript.items()[0].body).unwrap(),
        body
    );
}

#[test]
fn old_items_and_rule_reasons_remain_unchanged() {
    for text in [
        include_str!("../fixtures/record/item/toolCall.json"),
        include_str!("../fixtures/record/item/permissionReceipt.json"),
        include_str!("../fixtures/record/item/questionAnswer.json"),
    ] {
        replay(serde_json::from_str(text).unwrap());
    }
    let old: ItemBody =
        serde_json::from_str(include_str!("../fixtures/record/item/toolCall.json")).unwrap();
    assert!(matches!(
        old,
        ItemBody::ToolCall {
            asked_at: None,
            began_at: None,
            ..
        }
    ));
    let wire = json!({"kind":"rule","rule":"Edit"});
    let reason: Reason = serde_json::from_value(wire.clone()).unwrap();
    assert_eq!(serde_json::to_value(reason).unwrap(), wire);
}
