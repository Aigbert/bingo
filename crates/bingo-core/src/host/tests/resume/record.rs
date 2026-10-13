use super::*;

#[tokio::test]
async fn a_journal_with_unknown_records_resumes_with_known_history() {
    let wire = include_str!("../../../../../bingo-store-jsonl/fixtures/unknown-both.jsonl");
    let mut frames: Vec<Frame> = wire
        .lines()
        .skip(1)
        .map(|line| serde_json::from_str(line).expect("compatible frame"))
        .collect();
    let head = summary(frames[0].session.as_str());
    frames[0].event = Event::SessionUpdated {
        summary: head.clone(),
    };
    let store = Arc::new(crate::journal::MemoryStore::new());
    store.create(&head).await.unwrap();
    for frame in &frames {
        store.append(&head.id, frame).await.unwrap();
    }
    let provider = ScriptedProvider::new(vec![Script::Events(text("new answer"))]);
    let host = host_on(store, provider.clone()).await;
    let mut opened = host
        .open(
            SessionSelector::ById { id: head.id },
            who(),
            OpenOptions::default(),
        )
        .await
        .unwrap();
    assert!(
        opened
            .snapshot
            .transcript
            .items()
            .iter()
            .any(|item| matches!(item.body, ItemBody::Unknown { .. }))
    );
    assert!(
        opened.snapshot.transcript.items().iter().any(
            |item| matches!(&item.body, ItemBody::Assistant { text } if text == "known history")
        )
    );
    one_turn(&mut opened, "continue").await;
    let requests = provider.requests();
    let messages = &requests[0].messages;
    assert!(
        messages
            .iter()
            .any(|m| m.parts.iter().any(|p| p.as_text() == Some("known history")))
    );
    assert!(
        !serde_json::to_string(messages)
            .unwrap()
            .contains("futureItem")
    );
}
