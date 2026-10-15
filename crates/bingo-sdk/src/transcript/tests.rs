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

#[test]
fn open_items_belong_to_the_turn_and_exclude_terminal_or_removed_items() {
    let turn = TurnId::from_raw("owned");
    let mut transcript = Transcript::new();
    for (id, status) in [
        ItemStatus::Pending,
        ItemStatus::Running,
        ItemStatus::Completed,
        ItemStatus::Interrupted,
        ItemStatus::Failed,
    ]
    .into_iter()
    .enumerate()
    {
        let mut row = item(id as u8, String::new());
        row.turn = Some(turn.clone());
        row.status = status;
        transcript.apply(&Event::ItemStarted { item: row });
    }
    let mut other = item(9, String::new());
    other.status = ItemStatus::Running;
    transcript.apply(&Event::ItemStarted { item: other });
    let ids = |t: &Transcript| {
        t.open_items(&turn)
            .map(|i| i.id.clone())
            .collect::<Vec<_>>()
    };
    assert_eq!(
        ids(&transcript),
        [ItemId::from_raw("i0"), ItemId::from_raw("i1")]
    );
    transcript.apply(&Event::TurnRetrying {
        turn: turn.clone(),
        attempt: 1,
        max: 2,
        delay_ms: 0,
        reason: "retry".into(),
        dropped: vec![ItemId::from_raw("i0")],
    });
    assert_eq!(ids(&transcript), [ItemId::from_raw("i1")]);
}

#[test]
fn checkpoint_window_keeps_model_only_items_out_of_client_history() {
    let mut transcript: Transcript = (1..=4).map(|id| item(id, id.to_string())).collect();
    transcript.apply(&Event::Compacted {
        generation: 1,
        boundary: ItemId::from_raw("i3"),
        kept: vec![ItemId::from_raw("i1")],
        summary: ItemId::from_raw("i4"),
    });
    let before: Vec<_> = transcript.model_items().cloned().collect();
    transcript.drop_before_cut();
    assert_eq!(
        transcript.model_items().cloned().collect::<Vec<_>>(),
        before
    );
    assert_eq!(
        transcript
            .items()
            .iter()
            .map(|i| i.id.as_str())
            .collect::<Vec<_>>(),
        ["i3", "i4"]
    );
    assert!(transcript.get(&ItemId::from_raw("i1")).is_some());
    assert!(transcript.get(&ItemId::from_raw("i2")).is_none());
    assert_eq!(
        transcript.history(),
        OpenHistory {
            before: Some(ItemId::from_raw("i3")),
            has_more: true,
            generation: 1
        }
    );
}

#[test]
fn checkpoint_window_can_restore_full_client_order_without_losing_live_bodies() {
    let mut full: Transcript = (1..=4).map(|id| item(id, id.to_string())).collect();
    full.apply(&Event::Compacted {
        generation: 1,
        boundary: ItemId::from_raw("i3"),
        kept: vec![ItemId::from_raw("i1")],
        summary: ItemId::from_raw("i4"),
    });
    let mut window = full.clone();
    window.drop_before_cut();
    let restored = window.with_history(full.clone());
    assert_eq!(restored, full);
    let mut cut_again = restored;
    cut_again.drop_before_cut();
    assert_eq!(cut_again, window);
    let bytes = serde_json::to_value(&window).unwrap();
    assert_eq!(
        bytes.as_object().unwrap().len(),
        2,
        "client wire remains unchanged"
    );
    assert_eq!(bytes["items"].as_array().unwrap().len(), 2);
}

#[test]
fn turns_keep_full_prompts_and_share_the_cut_boundaries() {
    let mut first = item(1, String::new());
    first.turn = Some(TurnId::from_raw("t1"));
    first.body = ItemBody::User {
        parts: vec![ContentPart::text("first\nsecond")],
        origin: Origin::surface("terminal"),
    };
    let between = item(2, "notice".into());
    let mut last = item(3, "automatic".into());
    last.turn = Some(TurnId::from_raw("t2"));
    let transcript: Transcript = [first, between, last].into_iter().collect();
    let rows = turns(&transcript);
    assert_eq!(rows[0].asked.as_deref(), Some("first\nsecond"));
    assert_eq!(rows[0].label(), "first");
    assert_eq!((rows[0].first, rows[0].last, rows[0].items()), (0, 1, 2));
    assert_eq!(rows[1].label(), "t2");
    assert_eq!(from(&rows, &TurnId::from_raw("t1")).unwrap().len(), 2);
    assert!(from(&rows, &TurnId::from_raw("missing")).is_none());
}
#[test]
fn search_reads_full_results_reasoning_and_only_retained_items() {
    let mut tool = item(1, String::new());
    tool.turn = Some(TurnId::from_raw("t"));
    tool.body = ItemBody::ToolCall {
        call_id: "c".into(),
        name: "Read".into(),
        input: serde_json::json!({"file_path":"file"}),
        output: Some(ToolOutput::text("one\ntwo\nthree\nfour\nfive\nFORK")),
        progress: None,
        duration_ms: None,
        quiet: false,
    };
    let mut reasoning = item(2, String::new());
    reasoning.body = ItemBody::Reasoning {
        text: "a fork\nFORK again".into(),
        provider_metadata: Default::default(),
    };
    let mut transcript: Transcript = [tool, reasoning].into_iter().collect();
    assert!(find(&transcript, "").is_empty());
    let matches = find(&transcript, "Fork");
    assert_eq!(
        matches.iter().map(|m| m.line).collect::<Vec<_>>(),
        [8, 0, 1]
    );
    transcript.apply(&Event::Rewound {
        generation: 1,
        to_turn: TurnId::from_raw("t"),
        dropped: vec![ItemId::from_raw("i1")],
        files_restored: vec![],
    });
    assert_eq!(find(&transcript, "fork").len(), 2);
}
#[test]
fn asked_uses_effective_voice_and_borrows_only_the_first_text_part() {
    let mut row = item(1, String::new());
    row.body = ItemBody::Shell {
        command: "echo no".into(),
        output: "no".into(),
        exit: Some(0),
        cwd: ".".into(),
    };
    assert!(crate::asked(&row).is_none());
    row.body = ItemBody::User {
        parts: vec![ContentPart::text("/guide the wire\nexpanded prompt")],
        origin: Origin::surface("command"),
    };
    assert_eq!(crate::asked(&row), None);
    let legacy: std::collections::BTreeMap<String, Voice> =
        serde_json::from_str(include_str!("../asked-legacy.json")).unwrap();
    assert_eq!(legacy.len(), 6);
    for surface in legacy.into_keys() {
        row.body = ItemBody::User {
            parts: vec![
                ContentPart::text("first\nsecond"),
                ContentPart::text("ignored"),
            ],
            origin: Origin::surface(surface).voiced(Voice::Said),
        };
        assert_eq!(crate::asked(&row), Some("first\nsecond"));
        assert_eq!(crate::first_ask(&row).as_deref(), Some("first"));
        if let ItemBody::User { origin, .. } = &mut row.body {
            origin.voice = None;
        }
        assert!(crate::asked(&row).is_none());
    }
    for voice in [Voice::Note, Voice::Quiet, Voice::Aside] {
        row.body = ItemBody::User {
            parts: vec![ContentPart::text("injected text")],
            origin: Origin::surface("test").voiced(voice),
        };
        assert!(crate::asked(&row).is_none());
    }
}

#[test]
fn shell_lines_neither_open_turns_nor_replace_the_asked_prompt() {
    let mut shell = item(2, String::new());
    shell.body = ItemBody::Shell {
        command: "echo shell".into(),
        output: "shell".into(),
        exit: Some(0),
        cwd: ".".into(),
    };
    assert!(turns(&[shell.clone()].into_iter().collect()).is_empty());
    let mut user = item(1, String::new());
    user.turn = Some(TurnId::from_raw("t"));
    shell.turn = user.turn.clone();
    user.body = ItemBody::User {
        parts: vec![ContentPart::text("why is the build red?")],
        origin: Origin::surface("terminal"),
    };
    let rows = turns(&[user, shell].into_iter().collect());
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].label(), "why is the build red?");
}

#[test]
fn search_caps_results_and_reads_tool_input_as_lines() {
    let mut row = item(1, String::new());
    row.body = ItemBody::ToolCall {
        call_id: "c".into(),
        name: "Write".into(),
        input: serde_json::json!({"content":"first needle\nsecond needle", "file_path":"note.txt"}),
        output: None,
        progress: None,
        duration_ms: None,
        quiet: false,
    };
    let transcript = [row].into_iter().collect();
    let hits = find(&transcript, "needle");
    assert_eq!(hits.iter().map(|hit| hit.line).collect::<Vec<_>>(), [2, 3]);
    let transcript = [item(2, "needle\n".repeat(1001))].into_iter().collect();
    assert_eq!(find(&transcript, "needle").len(), 1000);
}
