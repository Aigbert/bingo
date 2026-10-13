use super::*;
use crate::*;
use jiff::Timestamp;
use proptest::prelude::*;

fn item(id: u8, text: String) -> Item {
    Item {
        id: ItemId::from_raw(format!("i{id}")),
        turn: None,
        round: 0,
        status: ItemStatus::Completed,
        started_at: Timestamp::UNIX_EPOCH,
        completed_at: None,
        intent: None,
        meta: Default::default(),
        body: ItemBody::Assistant { text },
    }
}

fn event((kind, id, other): (u8, u8, u8)) -> Event {
    let id_of = |n| ItemId::from_raw(format!("i{n}"));
    let item = item(id, other.to_string());
    match kind {
        0 => Event::ItemStarted { item },
        1 => Event::ItemUpdated { item },
        2 => Event::ItemCompleted { item },
        3 => Event::TurnRetrying {
            turn: TurnId::from_raw("t"),
            attempt: 1,
            max: 2,
            delay_ms: 0,
            reason: "retry".into(),
            dropped: vec![id_of(id)],
        },
        4 => Event::Rewound {
            generation: u64::from(other),
            to_turn: TurnId::from_raw("t"),
            dropped: vec![id_of(id)],
            files_restored: vec![],
        },
        _ => Event::Compacted {
            generation: u64::from(other),
            boundary: id_of(id),
            kept: vec![id_of(other)],
            summary: id_of((other + 1) % 8),
        },
    }
}

// Frozen from SessionState before M112: the client keeps compaction rows.
fn reference(items: &mut Vec<Item>, event: &Event) {
    match event {
        Event::ItemStarted { item }
        | Event::ItemUpdated { item }
        | Event::ItemCompleted { item } => match items.iter_mut().rev().find(|i| i.id == item.id) {
            Some(slot) => *slot = item.clone(),
            None => items.push(item.clone()),
        },
        Event::TurnRetrying { dropped, .. } | Event::Rewound { dropped, .. } => {
            items.retain(|i| !dropped.contains(&i.id));
        }
        _ => {}
    }
}

fn state() -> SessionState {
    let summary = serde_json::from_value(serde_json::json!({
        "id": "s", "cwd": "/tmp", "driver": "model",
        "createdAt": "1970-01-01T00:00:00Z", "updatedAt": "1970-01-01T00:00:00Z",
        "usage": Usage::default(), "busy": false,
    }))
    .unwrap();
    SessionState::new(summary)
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(1000))]
    #[test]
    fn transcript_matches_client_and_replay(codes in prop::collection::vec((0u8..7, 0u8..8, 0u8..8), 1..100)) {
        let events: Vec<_> = codes.into_iter().map(event).collect();
        let mut transcript = Transcript::new();
        let mut expected = Vec::new();
        let mut state = state();
        for (at, event) in events.iter().enumerate() {
            transcript.apply(event);
            reference(&mut expected, event);
            state.apply(&Frame { seq: Seq(at as u64 + 1), ts: Timestamp::UNIX_EPOCH,
                session: SessionId::from_raw("s"), cause: None, event: event.clone() });
            prop_assert_eq!(state.transcript.items(), transcript.items());
            prop_assert_eq!(transcript.items().iter().map(|i| i.as_ref()).collect::<Vec<_>>(), expected.iter().collect::<Vec<_>>());
            let mut replay = Transcript::new();
            for event in &events[..=at] { replay.apply(event); }
            prop_assert_eq!(&transcript, &replay);
            if let Event::TurnRetrying { dropped, .. } | Event::Rewound { dropped, .. } = event {
                prop_assert!(transcript.model_items().all(|i| !dropped.contains(&i.id)));
                prop_assert!(dropped.iter().all(|id| transcript.get(id).is_none()));
            }
        }
    }
}

#[test]
fn ten_thousand_items_share_bodies_in_a_snapshot() {
    let mut transcript = Transcript::new();
    for n in 0..10_000 {
        let mut item = item(0, "a body".repeat(100));
        item.id = ItemId::from_raw(format!("i{n}"));
        transcript.apply(&Event::ItemCompleted { item });
    }
    assert_eq!(transcript.items().len(), 10_000);
    let snapshot = transcript.clone();
    for (original, shared) in transcript.items().iter().zip(snapshot.items()) {
        assert!(Arc::ptr_eq(original, shared));
        assert_eq!(Arc::strong_count(original), 2);
    }
}

#[test]
fn a_delta_does_not_mutate_an_existing_snapshot() {
    let mut transcript = Transcript::new();
    let mut started = item(1, "hello".into());
    started.status = ItemStatus::Running;
    transcript.apply(&Event::ItemStarted { item: started });
    let snapshot = transcript.clone();
    let id = ItemId::from_raw("i1");
    transcript.apply(&Event::ItemDelta {
        item: id.clone(),
        n: 1,
        kind: DeltaKind::Text,
        data: " world".into(),
    });
    assert_eq!(
        snapshot.get(&id).unwrap().body,
        ItemBody::Assistant {
            text: "hello".into()
        }
    );
    assert_eq!(
        transcript.get(&id).unwrap().body,
        ItemBody::Assistant {
            text: "hello world".into()
        }
    );
    assert!(!Arc::ptr_eq(&snapshot.items()[0], &transcript.items()[0]));
}

#[test]
fn client_snapshot_deserialization_rebuilds_the_lookup() {
    let mut transcript: Transcript = [item(1, "one".into()), item(2, "two".into())]
        .into_iter()
        .collect();
    transcript.apply(&Event::Rewound {
        generation: 4,
        to_turn: TurnId::from_raw("t"),
        dropped: vec![ItemId::from_raw("i1")],
        files_restored: vec![],
    });
    let bytes = serde_json::to_vec(&transcript).unwrap();
    let restored: Transcript = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(restored.generation(), 4);
    assert_eq!(
        restored.get(&ItemId::from_raw("i2")),
        transcript.get(&ItemId::from_raw("i2"))
    );
    assert_eq!(serde_json::to_vec(&restored).unwrap(), bytes);
}
