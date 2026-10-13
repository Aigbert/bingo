//! A newer record survives an old client's real JSON-RPC process boundary.
#![allow(clippy::unwrap_used, clippy::expect_used)]

use bingo_sdk::{Event, Frame, HostApi, ItemBody, OpenOptions, Seq, SessionSelector};
use futures::StreamExt;

mod support;

#[tokio::test(flavor = "multi_thread")]
async fn unknown_records_resume_and_replay_over_rpc() {
    let mut server = support::Server::spawn(r#"{"responses":[]}"#);
    let kernel = support::ready(&mut server).await;
    let frames = plant(&server);
    let opened = kernel
        .open(
            SessionSelector::ById {
                id: frames[0].session.clone(),
            },
            support::who(),
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
            .any(|i| matches!(i.body, ItemBody::Unknown { .. }))
    );
    assert!(
        opened
            .snapshot
            .transcript
            .items()
            .iter()
            .any(|i| matches!(&i.body, ItemBody::Assistant { text } if text == "known history"))
    );
    let mut stream = opened.handle.events_since(Seq::ZERO).await.unwrap();
    let mut got = std::collections::BTreeMap::new();
    while got.len() < frames.len() {
        let frame = tokio::time::timeout(support::LIMIT, stream.next())
            .await
            .unwrap()
            .unwrap();
        // The attachment may already have buffered the new process's head.
        if frame.seq.0 <= frames.last().unwrap().seq.0 {
            got.insert(frame.seq.0, frame);
        }
    }
    assert!(matches!(got[&2].event, Event::Unknown { .. }));
    assert_eq!(
        serde_json::to_value(&got[&2]).unwrap(),
        serde_json::to_value(&frames[1]).unwrap()
    );
    assert!(
        matches!(&got[&3].event, Event::ItemCompleted { item } if matches!(item.body, ItemBody::Unknown { .. }))
    );
    kernel.shutdown().await.unwrap();
    assert!(
        tokio::time::timeout(support::LIMIT, server.child.wait())
            .await
            .unwrap()
            .unwrap()
            .success()
    );
}

fn plant(server: &support::Server) -> Vec<Frame> {
    let wire = include_str!("../../bingo-store-jsonl/fixtures/unknown-both.jsonl");
    let mut lines: Vec<serde_json::Value> = wire
        .lines()
        .map(|line| serde_json::from_str(line).unwrap())
        .collect();
    let summary = &mut lines[1]["event"]["summary"];
    summary["provider"] = "fake".into();
    summary["model"] = "fake-1".into();
    summary["cwd"] = serde_json::to_value(server.cwd()).unwrap();
    let id = lines[0]["session"].as_str().unwrap();
    let dir = server.sessions_dir().join(id);
    std::fs::create_dir_all(&dir).unwrap();
    let encoded = lines
        .iter()
        .map(|line| serde_json::to_string(line).unwrap())
        .collect::<Vec<_>>()
        .join("\n")
        + "\n";
    std::fs::write(dir.join("journal.jsonl"), encoded).unwrap();
    lines
        .into_iter()
        .skip(1)
        .map(|value| serde_json::from_value(value).unwrap())
        .collect()
}
