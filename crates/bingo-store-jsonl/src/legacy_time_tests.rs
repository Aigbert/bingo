use super::*;
use bingo_sdk::{ItemBody, SessionState, TurnId};

#[tokio::test]
async fn recorded_pre_m201_frames_replay_without_provenance_or_waits() {
    let root = tempfile::tempdir().unwrap();
    let store = JsonlStore::new(root.path());
    let id = SessionId::from_raw("ses_01M15S3S7ZCR0PJDTP4QH86XGX");
    let mut summary = tests::summary();
    summary.id = id.clone();
    store.create(&summary).await.unwrap();
    let source = legacy_journal();
    let header = format!("{{\"format\":\"bingo-journal\",\"version\":1,\"session\":\"{id}\"}}\n");
    let path = layout::journal(&layout::session_dir(store.root(), &id));
    std::fs::write(&path, format!("{header}{source}")).unwrap();
    let frames = store.replay(&id, Seq::ZERO).await.unwrap();
    assert_eq!(frames.len(), source.lines().count());
    assert!(matches!(
        frames.last().unwrap().event,
        bingo_sdk::Event::TurnCompleted { .. }
    ));
    let mut state = SessionState::new(summary);
    for frame in &frames {
        state.apply(frame);
    }
    assert_eq!(state.transcript.items().len(), 8);
    let mut waits = 0;
    for item in state.transcript.items() {
        match &item.body {
            ItemBody::ToolCall { allowed_by, .. } => assert!(allowed_by.is_none()),
            ItemBody::PermissionReceipt { asked_at, .. }
            | ItemBody::QuestionAnswer { asked_at, .. }
            | ItemBody::Interruption { asked_at, .. } => {
                assert!(asked_at.is_none());
                waits += 1;
            }
            _ => {}
        }
        let wire = serde_json::to_value(&item.body).unwrap();
        assert!(wire.get("allowedBy").is_none() && wire.get("askedAt").is_none());
    }
    assert_eq!(waits, 3);
    let legacy =
        bingo_sdk::transcript::clock(&state, &TurnId::from_raw("legacy-waits"), Timestamp::now())
            .unwrap();
    assert_eq!(legacy.elapsed, jiff::SignedDuration::from_secs(2));
    assert_eq!(legacy.next_second, None);
    let turn = TurnId::from_raw("trn_01M15S3S81MG6136V62EPMFZ4T");
    let clock = bingo_sdk::transcript::clock(&state, &turn, Timestamp::now()).unwrap();
    assert_eq!(clock.elapsed, jiff::SignedDuration::from_micros(6039));
    assert_eq!(clock.next_second, None);
    assert_eq!(
        std::fs::read_to_string(path).unwrap(),
        format!("{header}{source}")
    );
}

fn legacy_journal() -> String {
    let mut source = include_str!("../../bingo-core/fixtures/frames-v1.jsonl").to_string();
    let last: serde_json::Value = serde_json::from_str(source.lines().last().unwrap()).unwrap();
    let mut seq = last["seq"].as_u64().unwrap();
    let mut frame = |event: String, second: u32| {
        seq += 1;
        source.push_str(&format!(r#"{{"seq":{seq},"ts":"2026-08-30T00:00:0{second}Z","session":"ses_01M15S3S7ZCR0PJDTP4QH86XGX","event":{event}}}
"#));
    };
    frame(
        r#"{"type":"turnStarted","turn":"legacy-waits","inputs":[],"origin":"submit"}"#.into(),
        0,
    );
    for (n, body) in [
        include_str!("../../bingo-sdk/fixtures/record/item/permissionReceipt.json"),
        include_str!("../../bingo-sdk/fixtures/record/item/questionAnswer.json"),
        include_str!("../../bingo-sdk/fixtures/record/item/interruption.json"),
    ]
    .iter()
    .enumerate()
    {
        let body = body.trim_end();
        frame(
            format!(
                r#"{{"type":"itemCompleted","item":{{"id":"legacy-{n}","turn":"legacy-waits","status":"completed","startedAt":"2026-08-30T00:00:0{n}Z","completedAt":"2026-08-30T00:00:0{n}Z","body":{body}}}}}"#
            ),
            n as u32,
        );
    }
    frame(r#"{"type":"turnCompleted","turn":"legacy-waits","status":{"kind":"completed"},"usage":{"inputTokens":0,"outputTokens":0,"cacheReadTokens":0,"cacheWriteTokens":0,"reasoningTokens":0}}"#.into(), 2);
    source
}
