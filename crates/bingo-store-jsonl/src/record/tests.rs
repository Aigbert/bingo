use super::*;
use bingo_sdk::{Event, ItemBody, SessionState};

fn pictured() -> Frame {
    serde_json::from_str(
        include_str!("../../fixtures/picture-v1.jsonl")
            .lines()
            .nth(1)
            .unwrap(),
    )
    .unwrap()
}

#[test]
fn nested_parts_are_blobs_but_image_shaped_tool_input_is_opaque() {
    let mut frame = pictured();
    let Event::ItemCompleted { item } = &mut frame.event else {
        panic!("item")
    };
    let ItemBody::User { parts, .. } = &mut item.body else {
        panic!("user")
    };
    let image = parts.remove(0);
    let input =
        serde_json::json!({"type": "image", "mediaType": "image/png", "data": "not base64"});
    parts.extend([
        ContentPart::ToolUse {
            id: "call".into(),
            name: "Read".into(),
            input,
        },
        ContentPart::ToolResult {
            tool_use_id: "call".into(),
            parts: vec![image],
            is_error: true,
        },
    ]);
    let mut puts = 0;
    let encoded = encode(&frame, &mut |bytes| {
        puts += 1;
        assert_eq!(bytes, b"x");
        Ok("sha".into())
    })
    .unwrap();
    assert_eq!(puts, 1);
    assert!(encoded.contains("not base64"));
    let restored = decode(encoded.as_bytes(), &mut |sha| {
        assert_eq!(sha, "sha");
        Some(b"x".to_vec())
    })
    .unwrap();
    assert_eq!(restored, frame);
}

#[test]
fn a_checkpoint_with_pictures_keeps_unknown_sibling_bytes() {
    let mut state = SessionState::new(crate::tests::summary());
    let frame = pictured();
    state.apply(&frame);
    let Event::ItemCompleted { mut item } = frame.event else {
        panic!("item")
    };
    item.id = bingo_sdk::ItemId::mint();
    let raw = r#"{ "kind" : "user", "parts": [{"type":"image","mediaType":"image/png","data":"eA=="}], "missing_origin" : [1,  2] }"#;
    item.body = serde_json::from_str(raw).unwrap();
    state.apply(&crate::tests::frame(2, Event::ItemCompleted { item }));
    let checkpoint = Checkpoint::new(state);
    let bytes = encode_checkpoint(&checkpoint, &mut |_| Ok("sha".into())).unwrap();
    assert!(std::str::from_utf8(&bytes).unwrap().contains(raw));
    assert_eq!(
        decode_checkpoint(&bytes, &mut |_| Some(b"x".to_vec())).unwrap(),
        checkpoint
    );
}
