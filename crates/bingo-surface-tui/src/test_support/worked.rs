//! Turns that did some work (ADR-0095 §6–§8): the prototype's own, at each
//! moment a person sees one — running, finished, stopped — and a long run of
//! them.

use bingo_sdk::{
    Event, Frame, InterruptReason, Item, ItemBody, ItemStatus, ToolOutput, TurnStatus, Upshot,
};
use serde_json::json;

use super::{assistant, diff_output, frame, item, started, tool, user};

/// The prototype's turn in `turn`: a thought, five steps, a word between them
/// and the answer after them. Ids are `{turn}_{n}`.
pub fn worked(turn: &str) -> Vec<Item> {
    let done = ItemStatus::Completed;
    let id = |n: u32| format!("{turn}_{n}");
    let said = |text: &str, upshot: Upshot| ToolOutput {
        upshot: Some(upshot),
        ..ToolOutput::text(text)
    };
    let thought = ItemBody::Reasoning {
        text: "The total is printed bare; main is where it is said.".into(),
        provider_metadata: Default::default(),
    };
    let read = ToolOutput::text("fn main() {\n    println!(\"{}\", total());\n}");
    let items = vec![
        user(&id(0), "label the total in main"),
        item(&id(1), done, thought),
        tool(&id(2), "Read", path("src/main.rs"), Some(read), done),
        tool(
            &id(3),
            "Grep",
            json!({"pattern": "total"}),
            Some(said("src/main.rs", Upshot::plain("1 file"))),
            done,
        ),
        assistant(&id(4), "One place prints it.", done),
        tool(
            &id(5),
            "Edit",
            path("src/main.rs"),
            Some(diff_output()),
            done,
        ),
        tool(
            &id(6),
            "Bash",
            json!({"command": "cargo test -q"}),
            Some(said("test result: ok. 3 passed", Upshot::good("3 passed"))),
            done,
        ),
        assistant(&id(7), "Done. main now prints a labelled total.", done),
    ];
    items
        .into_iter()
        .zip(0..)
        .map(|(item, second)| at(in_turn(called(item), turn), second))
        .collect()
}

/// A call named after its item, as every call of a session is named apart.
fn called(mut item: Item) -> Item {
    if let ItemBody::ToolCall { call_id, .. } = &mut item.body {
        *call_id = format!("call_{}", item.id);
    }
    item
}

/// An item started `second`s into its turn and done a second later.
fn at(item: Item, second: i64) -> Item {
    let started_at = super::ts() + jiff::SignedDuration::from_secs(second);
    let completed_at = item
        .completed_at
        .map(|_| started_at + jiff::SignedDuration::from_secs(1));
    Item {
        started_at,
        completed_at,
        ..item
    }
}

/// The prototype's turn as the stream told it, over.
pub fn finished_turn() -> Vec<Frame> {
    told("trn_1", worked("trn_1"), TurnStatus::Completed)
}

/// The prototype's turn caught on its fourth step, the test still running.
pub fn running_turn() -> Vec<Frame> {
    let mut items = worked("trn_1");
    items.truncate(7);
    if let Some(last) = items.last_mut() {
        last.status = ItemStatus::Running;
        last.completed_at = None;
        if let ItemBody::ToolCall {
            output, began_at, ..
        } = &mut last.body
        {
            *output = None;
            *began_at = Some(last.started_at);
        }
    }
    let mut frames = vec![frame(1, started("trn_1"))];
    frames.extend(numbered(2, items));
    frames
}

/// The prototype's turn stopped on its fourth step: three had run.
pub fn stopped_turn() -> Vec<Frame> {
    let mut items = worked("trn_1");
    items.truncate(7);
    if let Some(last) = items.last_mut() {
        last.status = ItemStatus::Interrupted;
        if let ItemBody::ToolCall {
            output, began_at, ..
        } = &mut last.body
        {
            *output = None;
            *began_at = Some(last.started_at);
        }
    }
    let marker = ItemBody::Interruption {
        asked_at: None,
        marker: "[Request interrupted by the user]".into(),
    };
    items.push(in_turn(
        item("trn_1_8", ItemStatus::Completed, marker),
        "trn_1",
    ));
    let stopped = TurnStatus::Interrupted {
        reason: InterruptReason::UserCancel,
    };
    told("trn_1", items, stopped)
}

/// `turns` finished turns, one after another, each the prototype's.
pub fn long_session(turns: usize) -> Vec<Frame> {
    (0..turns)
        .flat_map(|n| {
            let turn = format!("trn_{n}");
            told(&turn, worked(&turn), TurnStatus::Completed)
                .into_iter()
                .map(move |mut frame| {
                    let offset = jiff::SignedDuration::from_secs(n as i64 * 10);
                    frame.ts += offset;
                    if let Event::ItemCompleted { item } = &mut frame.event {
                        item.started_at += offset;
                        item.completed_at = item.completed_at.map(|at| at + offset);
                    }
                    frame
                })
        })
        .enumerate()
        .map(|(seq, frame)| Frame {
            seq: bingo_sdk::Seq(seq as u64 + 1),
            ..frame
        })
        .collect()
}

/// A turn's items between its start and its end.
fn told(turn: &str, items: Vec<Item>, status: TurnStatus) -> Vec<Frame> {
    let mut frames = vec![frame(1, started(turn))];
    let end = items.len() as u64 + 2;
    frames.extend(numbered(2, items));
    let mut done = frame(end, super::completed(turn, status.clone()));
    done.ts = super::ts()
        + jiff::SignedDuration::from_secs(if matches!(status, TurnStatus::Interrupted { .. }) {
            7
        } else {
            8
        });
    frames.push(done);
    frames
}

fn numbered(first: u64, items: Vec<Item>) -> Vec<Frame> {
    items
        .into_iter()
        .zip(first..)
        .map(|(item, seq)| {
            let at = item.completed_at.unwrap_or(item.started_at);
            let mut next = frame(seq, Event::ItemCompleted { item });
            next.ts = at;
            next
        })
        .collect()
}

fn in_turn(item: Item, turn: &str) -> Item {
    Item {
        turn: Some(bingo_sdk::TurnId::from_raw(turn)),
        ..item
    }
}

fn path(file: &str) -> serde_json::Value {
    json!({ "file_path": format!("/tmp/project/{file}") })
}
