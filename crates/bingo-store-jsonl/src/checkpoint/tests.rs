use super::super::*;
use crate::tests::{frame, root_with, session, summary};
use bingo_sdk::{Checkpoint, Item, ItemBody, ItemId, SessionState};

async fn journal_with_cut(store: &JsonlStore) -> SessionState {
    store.create(&summary()).await.unwrap();
    let mut state = SessionState::new(summary());
    let head = frame(1, Event::SessionUpdated { summary: summary() });
    store.append(&session(), &head).await.unwrap();
    state.apply(&head);
    for n in 1..=4 {
        let item: Item = serde_json::from_value(serde_json::json!({
            "id": format!("i{n}"), "round": 0, "status": "completed",
            "startedAt": "1970-01-01T00:00:00Z",
            "body": { "kind": "assistant", "text": n.to_string() }
        }))
        .unwrap();
        let frame = frame(n + 1, Event::ItemCompleted { item });
        store.append(&session(), &frame).await.unwrap();
        state.apply(&frame);
    }
    let cut = frame(
        6,
        Event::Compacted {
            generation: 1,
            boundary: ItemId::from_raw("i3"),
            kept: vec![ItemId::from_raw("i1")],
            summary: ItemId::from_raw("i4"),
        },
    );
    store.append(&session(), &cut).await.unwrap();
    state.apply(&cut);
    state
}

#[tokio::test]
async fn checkpoint_resume_reads_only_tail_and_preserves_the_fold() {
    let (_root, store) = root_with();
    let mut state = journal_with_cut(&store).await;
    state.transcript.drop_before_cut();
    store
        .checkpoint(&session(), &Checkpoint::new(state.clone()))
        .await
        .unwrap();
    let tail = frame(
        7,
        Event::SessionClosed {
            reason: bingo_sdk::CloseReason::Client,
        },
    );
    store.append(&session(), &tail).await.unwrap();
    let resumed = store.resume(&session()).await.unwrap();
    assert_eq!(resumed.state, Some(state));
    assert_eq!(resumed.tail, [tail]);
    let mut full = SessionState::new(summary());
    for frame in store.replay(&session(), Seq::ZERO).await.unwrap() {
        full.apply(&frame);
    }
    full.transcript.drop_before_cut();
    let mut restored = resumed.state.unwrap();
    for frame in resumed.tail {
        restored.apply(&frame);
    }
    assert_eq!(restored, full);
}

#[tokio::test]
async fn checkpoint_corruption_replays_and_repairs() {
    for damage in [
        "missing",
        "json",
        "version",
        "session",
        "hash",
        "offset",
        "truncated",
        "edited",
    ] {
        let (_root, store) = root_with();
        let state = journal_with_cut(&store).await;
        store
            .checkpoint(&session(), &Checkpoint::new(state))
            .await
            .unwrap();
        let dir = layout::session_dir(store.root(), &session());
        let path = dir.join("checkpoint.json");
        damage_checkpoint(damage, &dir, &path);
        let expected = store.replay(&session(), Seq::ZERO).await.unwrap();
        let resumed = store.resume(&session()).await.unwrap();
        assert!(resumed.state.is_none(), "{damage}");
        assert_eq!(resumed.tail, expected, "{damage}");
        assert!(
            store.resume(&session()).await.unwrap().state.is_some(),
            "repair {damage}"
        );
    }
}

fn damage_checkpoint(damage: &str, dir: &std::path::Path, path: &std::path::Path) {
    match damage {
        "missing" => std::fs::remove_file(path).unwrap(),
        "json" => std::fs::write(path, "{").unwrap(),
        "truncated" => {
            let bytes = std::fs::read(layout::journal(dir)).unwrap();
            let end = bytes[..bytes.len() - 1]
                .iter()
                .rposition(|b| *b == b'\n')
                .unwrap()
                + 1;
            std::fs::write(layout::journal(dir), &bytes[..end]).unwrap();
        }
        "edited" => {
            let mut frames = journal::replay(dir, Seq::ZERO).unwrap();
            frames.last_mut().unwrap().event = Event::ItemCompleted {
                item: Item {
                    body: ItemBody::Assistant {
                        text: "edited".into(),
                    },
                    ..serde_json::from_str::<Checkpoint>(&std::fs::read_to_string(path).unwrap())
                        .unwrap()
                        .state
                        .transcript
                        .items()[0]
                        .as_ref()
                        .clone()
                },
            };
            let text = std::fs::read_to_string(layout::journal(dir)).unwrap();
            let header = text.lines().next().unwrap();
            let mut replaced = format!("{header}\n");
            for frame in frames {
                replaced.push_str(&serde_json::to_string(&frame).unwrap());
                replaced.push('\n');
            }
            std::fs::write(layout::journal(dir), replaced).unwrap();
        }
        _ => {
            let mut cp: serde_json::Value =
                serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap();
            match damage {
                "version" => cp["version"] = 0.into(),
                "session" => cp["state"]["session"]["summary"]["id"] = "foreign".into(),
                "hash" => cp["lineHash"] = "wrong".into(),
                "offset" => cp["offset"] = u64::MAX.into(),
                _ => panic!("unknown damage"),
            }
            std::fs::write(path, serde_json::to_vec(&cp).unwrap()).unwrap();
        }
    }
}

#[tokio::test]
async fn checkpoint_resume_seeks_past_the_prefix_and_reads_versioned_tail() {
    let (_root, store) = root_with();
    let mut state = journal_with_cut(&store).await;
    state.transcript.drop_before_cut();
    store
        .checkpoint(&session(), &Checkpoint::new(state.clone()))
        .await
        .unwrap();
    let dir = layout::session_dir(store.root(), &session());
    journal::append_version(&dir).unwrap();
    let tail = frame(
        7,
        Event::SessionClosed {
            reason: bingo_sdk::CloseReason::Client,
        },
    );
    store.append(&session(), &tail).await.unwrap();
    // Preserve offsets and the anchor; a prefix parse would now fail.
    let mut bytes = std::fs::read(layout::journal(&dir)).unwrap();
    let first_frame = bytes.iter().position(|b| *b == b'\n').unwrap() + 1;
    bytes[first_frame] = b'!';
    std::fs::write(layout::journal(&dir), bytes).unwrap();
    assert!(store.replay(&session(), Seq::ZERO).await.is_err());
    let resumed = store.resume(&session()).await.unwrap();
    assert_eq!(resumed.state, Some(state));
    assert_eq!(resumed.tail, [tail]);
}

#[tokio::test]
async fn checkpoint_writes_cannot_regress_to_an_older_sequence() {
    let (_root, store) = root_with();
    let mut state = journal_with_cut(&store).await;
    let old = Checkpoint::new(state.clone());
    let tail = frame(
        7,
        Event::SessionClosed {
            reason: bingo_sdk::CloseReason::Client,
        },
    );
    store.append(&session(), &tail).await.unwrap();
    state.apply(&tail);
    store
        .checkpoint(&session(), &Checkpoint::new(state))
        .await
        .unwrap();
    store.checkpoint(&session(), &old).await.unwrap();
    assert_eq!(
        store.resume(&session()).await.unwrap().state.unwrap().seq,
        Seq(7)
    );
}

#[tokio::test]
async fn a_version_one_checkpoint_is_replayed_after_the_bump() {
    let (_root, store) = root_with();
    let state = journal_with_cut(&store).await;
    store
        .checkpoint(&session(), &Checkpoint::new(state))
        .await
        .unwrap();
    let dir = layout::session_dir(store.root(), &session());
    let path = dir.join("checkpoint.json");
    let mut saved: serde_json::Value =
        serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
    saved["version"] = 1.into();
    std::fs::write(path, serde_json::to_vec(&saved).unwrap()).unwrap();
    assert!(store.resume(&session()).await.unwrap().state.is_none());
    assert!(store.resume(&session()).await.unwrap().state.is_some());
}

#[tokio::test]
async fn checkpoint_pictures_are_references_and_missing_blobs_are_notes() {
    let (_root, store) = root_with();
    let mut state = journal_with_cut(&store).await;
    let line = include_str!("../../fixtures/picture-v1.jsonl")
        .lines()
        .nth(1)
        .unwrap();
    let mut picture: bingo_sdk::Frame = serde_json::from_str(line).unwrap();
    picture.seq = Seq(7);
    picture.session = session();
    store.append(&session(), &picture).await.unwrap();
    state.apply(&picture);
    store
        .checkpoint(&session(), &Checkpoint::new(state.clone()))
        .await
        .unwrap();
    let dir = layout::session_dir(store.root(), &session());
    let saved = std::fs::read_to_string(dir.join("checkpoint.json")).unwrap();
    assert!(saved.contains("\"blob\""));
    assert!(!saved.contains("\"data\""));
    assert_eq!(store.resume(&session()).await.unwrap().state, Some(state));
    std::fs::remove_dir_all(dir.join("blobs")).unwrap();
    let resumed = store.resume(&session()).await.unwrap().state.unwrap();
    let item = resumed.transcript.get(&ItemId::from_raw("i")).unwrap();
    assert!(matches!(&item.body, ItemBody::User { parts, .. }
        if parts == &[bingo_sdk::ContentPart::text("[picture missing: image/png 0.0 KB]")]));
}
