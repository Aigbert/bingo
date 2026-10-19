//! A real journal fixes every receipt fact except the wall clock's sub-minute remainder.
use super::*;
use bingo_sdk::{Event, Frame, Item, ItemBody, ItemId, ItemStatus, SessionId, TurnId};

const ID: &str = "ses_01JB7Y2K9QH4X3V8M6N5P2R1TC";

fn seed(home: &std::path::Path) {
    let Event::SessionUpdated { mut summary } = serde_json::from_str(include_str!(
        "../../../bingo-sdk/fixtures/record/event/sessionUpdated.json"
    ))
    .unwrap() else {
        panic!("summary")
    };
    summary.id = SessionId::from_raw(ID);
    summary.cwd = home.display().to_string();
    summary.title = Some("steer 吞掉排队消息".into());
    let start = jiff::Timestamp::now() - jiff::SignedDuration::from_secs(9990);
    summary.created_at = start;
    summary.updated_at = start;
    // The palette's once-per-install key hint is a placed toast that would
    // race the resume toast for the one notice; this install has seen it.
    let hint = home.join(".bingo/data/plugins/palette/key-hint");
    std::fs::create_dir_all(hint.parent().unwrap()).unwrap();
    std::fs::write(&hint, "").unwrap();
    let dir = home.join(".bingo/data/sessions").join(ID);
    bingo_store_jsonl::journal::create(&dir, &summary.id).unwrap();
    bingo_store_jsonl::summary::write(&dir, &summary).unwrap();
    let mut seq = 0;
    let mut emit = |event: Event, ts| {
        seq += 1;
        bingo_store_jsonl::journal::append(
            &dir,
            &Frame {
                seq: bingo_sdk::Seq(seq),
                ts,
                session: summary.id.clone(),
                cause: None,
                event,
            },
        )
        .unwrap();
    };
    emit(
        Event::SessionUpdated {
            summary: summary.clone(),
        },
        start,
    );
    for n in 0..2 {
        let turn = TurnId::from_raw(format!("receipt-{n}"));
        emit(
            Event::TurnStarted {
                turn: turn.clone(),
                inputs: vec![],
                origin: bingo_sdk::TurnOrigin::Submit,
            },
            start,
        );
        let mut item: Item = serde_json::from_str(include_str!(
            "../../../bingo-sdk/fixtures/headline/item.json"
        ))
        .unwrap();
        item.meta.clear();
        item.id = ItemId::from_raw(format!("prompt-{n}"));
        item.turn = Some(turn.clone());
        item.started_at = start;
        item.completed_at = Some(start);
        item.body = ItemBody::User {
            parts: vec![bingo_sdk::ContentPart::text("hello")],
            origin: bingo_sdk::Origin::surface("tui"),
        };
        emit(Event::ItemCompleted { item: item.clone() }, start);
        item.id = ItemId::from_raw(format!("edit-{n}"));
        item.status = ItemStatus::Completed;
        item.body = serde_json::from_value(serde_json::json!({
            "kind":"toolCall", "callId":format!("call-{n}"), "name":"Write",
            "input":{"file_path":format!("file-{n}.rs"),"content":"new"},
            "output":{"parts":[],"upshot":{"kind":"diff","added":1,"removed":0}}
        }))
        .unwrap();
        emit(Event::ItemCompleted { item }, start);
        emit(
            Event::TurnCompleted {
                turn,
                status: bingo_sdk::TurnStatus::Completed,
                usage: Default::default(),
            },
            start,
        );
    }
}

#[test]
fn exit_receipt_matches_the_prototype_rows_byte_for_byte() {
    let mut terminal = Terminal::under_seeded(
        &["--resume", ID, "--no-print-on-exit"],
        SCRIPT,
        Answers::Da1Only,
        Ground::Eight,
        Some(seed),
    );
    terminal.wait_for("file-1.rs");
    terminal.wait_for("resumed · steer");
    terminal.send(&[0x03]);
    terminal.wait_for("again");
    terminal.send(&[0x03]);
    let shell = terminal.leave();
    let rows = shell
        .lines()
        .filter(|line| !line.is_empty())
        .collect::<Vec<_>>()
        .join("\n");
    assert_eq!(
        rows.as_bytes(),
        concat!(
            "● bingo · steer 吞掉排队消息\n",
            "  2 turns · 2h 46m · edited 2 files\n",
            "  resume with bingo --resume ses_01JB7Y2K9QH4X3V8M6N5P2R1TC"
        )
        .as_bytes()
    );
}
