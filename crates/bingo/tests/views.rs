//! Black-box: a plugin's three lanes (ADR-0013 §2) over the wire and through
//! the journal, with the hello fixture enabled explicitly. A panel is
//! durable and comes back for the next run; a signal is on the live stream
//! and nowhere else; a button fires a command and the table changes.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::time::Duration;

use bingo_sdk::{
    Action, Attachment, Event, HostApi, Input, IntentId, IntentOutcome, OpenOptions, Origin, Seq,
    SessionSelector, View,
};
use futures::StreamExt;

use crate::support;

use support::{LIMIT, Server, ack_for, create, ready, who};

/// Two fixture progress calls followed by one answer.
const PROGRESS_TURN: &str = r#"{"responses":[
    {"steps":[{"toolCall":{"name":"HelloProgress","input":{"value":0}}}]},
    {"steps":[{"toolCall":{"name":"HelloProgress","input":{"value":100}}}]},
    {"steps":[{"text":"The bar has run."}]}
]}"#;

const NOTHING: &str = r#"{"responses":[{"steps":[{"text":"nothing to do"}]}]}"#;

fn hello(home: tempfile::TempDir, script: &str) -> Server {
    let cwd = home.path().to_path_buf();
    Server::spawn_in_with_env(home, script, &[], &cwd, &[("BINGO_FAKE_HELLO", "1")])
}

fn intent(n: u8) -> IntentId {
    IntentId::from_raw(format!("req_01HVIEWS00000000000000000{n}"))
}

/// Submit and wait for the ack: a command is answered without a turn.
async fn command(attachment: &mut Attachment, n: u8, text: &str) -> IntentOutcome {
    let intent = intent(n);
    attachment
        .handle
        .submit(intent.clone(), Input::text(text, Origin::surface("rpc")));
    ack_for(attachment, &intent).await
}

/// Fire a button, which is what a surface does with a `View::Actions` item.
async fn fire(attachment: &mut Attachment, n: u8, name: &str) -> IntentOutcome {
    let intent = submit_action(attachment, n, name);
    ack_for(attachment, &intent).await
}

fn submit_action(attachment: &Attachment, n: u8, name: &str) -> IntentId {
    let intent = intent(n);
    attachment.handle.submit(
        intent.clone(),
        Input::Action {
            action: Action {
                name: name.into(),
                args: serde_json::Value::Null,
            },
            origin: Some(bingo_sdk::Origin::surface("tui")),
        },
    );
    intent
}

/// The board as the session has it, read as the vocabulary it is published in.
fn board(attachment: &Attachment) -> View {
    let payload = attachment
        .snapshot
        .extensions
        .get("hello")
        .and_then(|kinds| kinds.get("board"))
        .unwrap_or_else(|| panic!("no board in {:?}", attachment.snapshot.extensions));
    serde_json::from_value(payload.clone()).expect("a panel published in the vocabulary")
}

/// What the board's table says, row by row.
fn rows(view: &View) -> Vec<Vec<String>> {
    let View::Panel { child, .. } = view else {
        panic!("a board is a panel: {view:?}");
    };
    let View::Box { children, .. } = child.as_ref() else {
        panic!("a table and its buttons");
    };
    children
        .iter()
        .find_map(|child| match child {
            View::Table { rows, .. } => Some(rows.clone()),
            _ => None,
        })
        .expect("a table")
}

/// The names of the buttons the board offers.
fn buttons(view: &View) -> Vec<String> {
    let View::Panel { child, .. } = view else {
        panic!("a board is a panel");
    };
    let View::Box { children, .. } = child.as_ref() else {
        panic!("a table and its buttons");
    };
    children
        .iter()
        .find_map(|child| match child {
            View::Actions { items } => {
                Some(items.iter().map(|item| item.action.name.clone()).collect())
            }
            _ => None,
        })
        .expect("buttons")
}

#[tokio::test(flavor = "multi_thread")]
async fn a_button_fires_its_command_and_the_table_changes() {
    let mut server = hello(tempfile::tempdir().unwrap(), NOTHING);
    let kernel = ready(&mut server).await;
    let mut attachment = kernel
        .open(create(server.cwd()), who(), OpenOptions::default())
        .await
        .unwrap();

    let published = command(&mut attachment, 1, "/hello-board").await;
    assert!(
        matches!(published, IntentOutcome::Applied { .. }),
        "{published:?}"
    );
    let first = board(&attachment);
    assert_eq!(
        buttons(&first),
        ["hello.tick", "hello.reset"],
        "a button names the command a surface fires"
    );
    assert_eq!(rows(&first)[0][2], "pending");

    let ticked = fire(&mut attachment, 2, "hello.tick").await;
    assert!(
        matches!(ticked, IntentOutcome::Applied { .. }),
        "{ticked:?}"
    );
    assert_eq!(
        rows(&board(&attachment))[0][2],
        "running",
        "the row the button moved on"
    );

    let reset = fire(&mut attachment, 3, "hello.reset").await;
    assert!(matches!(reset, IntentOutcome::Applied { .. }), "{reset:?}");
    assert_eq!(rows(&board(&attachment))[0][2], "pending");
    kernel.shutdown().await.unwrap();
}

#[tokio::test(flavor = "multi_thread")]
async fn concurrent_buttons_preserve_both_changes_and_the_placed_row() {
    let mut server = hello(tempfile::tempdir().unwrap(), NOTHING);
    let kernel = ready(&mut server).await;
    let mut first = kernel
        .open(create(server.cwd()), who(), OpenOptions::default())
        .await
        .unwrap();
    command(&mut first, 1, "/hello-board").await;
    let waiting = vec![
        submit_action(&first, 2, "hello.tick"),
        submit_action(&first, 3, "hello.tick"),
    ];
    applied(&mut first, waiting).await;
    assert_eq!(rows(&board(&first))[0][2], "done");
    let placed = bingo_sdk::slot::placed(&first.snapshot);
    assert!(placed.iter().any(|placed| matches!(&placed.slot, bingo_sdk::slot::Slot::Row { body: Some(View::Table { rows, .. }), .. } if rows[0][2] == "done")));
    kernel.shutdown().await.unwrap();
}

/// The one lane that is not durable: a signal reaches a client and is left
/// out of the journal the next run replays.
#[tokio::test(flavor = "multi_thread")]
async fn a_signal_is_on_the_live_stream_and_never_in_the_journal() {
    let mut server = hello(tempfile::tempdir().unwrap(), PROGRESS_TURN);
    let kernel = ready(&mut server).await;
    let mut attachment = kernel
        .open(create(server.cwd()), who(), OpenOptions::default())
        .await
        .unwrap();
    command(&mut attachment, 1, "/hello-board").await;

    let intent = intent(2);
    attachment.handle.submit(
        intent.clone(),
        Input::text("run the bar", Origin::surface("test")),
    );
    let live = until_completed(&mut attachment).await;

    assert_progress_lanes(&attachment, &live);
    let reset = fire(&mut attachment, 3, "hello.reset").await;
    assert!(matches!(reset, IntentOutcome::Applied { .. }), "{reset:?}");
    assert!(
        attachment
            .snapshot
            .signals
            .get("hello")
            .is_none_or(|kinds| !kinds.contains_key("progress"))
    );

    kernel.shutdown().await.unwrap();
    assert!(server.child.wait().await.unwrap().success());
    assert_journal_lanes(&server, &attachment, &live);
}

fn assert_progress_lanes(attachment: &Attachment, live: &[bingo_sdk::Frame]) {
    let bars: Vec<&Event> = live
        .iter()
        .map(|frame| &frame.event)
        .filter(|event| matches!(event, Event::Signal { kind, .. } if kind == "progress"))
        .collect();
    assert!(bars.len() > 1, "the bar moved: {} frames", bars.len());
    assert!(
        attachment.snapshot.signals.contains_key("hello"),
        "and the latest frame is what a client shows"
    );

    assert!(live.iter().any(|frame| matches!(&frame.event,
        Event::ItemCompleted { item } if matches!(&item.body,
            bingo_sdk::ItemBody::ToolCall { name, output: Some(output), .. }
            if name == "HelloProgress" && matches!(output.display, Some(View::Progress { .. }))
        )
    )));
    for event in &bars {
        let Event::Signal { payload, .. } = event else {
            unreachable!()
        };
        assert_eq!(payload["slots"][0]["kind"], "activity");
    }
}

fn assert_journal_lanes(server: &Server, attachment: &Attachment, live: &[bingo_sdk::Frame]) {
    // events_since also tails live signals, even after TurnCompleted.
    let dir = server.sessions_dir().join(attachment.session.as_str());
    let journal = bingo_store_jsonl::journal::replay(&dir, Seq::ZERO).unwrap();
    assert!(
        journal.contains(live.last().unwrap()),
        "the journal contains the completed turn"
    );
    assert!(
        !journal
            .iter()
            .any(|frame| matches!(frame.event, Event::Signal { .. })),
        "a signal is never journaled"
    );
    assert!(
        journal.iter().any(
            |frame| matches!(&frame.event, Event::Extension { plugin, .. } if plugin == "hello")
        ),
        "and the panel beside it is"
    );
}

/// What `--continue` finds: the board the last run left, and no trace of the
/// bar that was moving while it ran.
#[tokio::test(flavor = "multi_thread")]
async fn a_second_run_reads_the_board_back_and_not_the_bar() {
    let mut first = hello(tempfile::tempdir().unwrap(), PROGRESS_TURN);
    let kernel = ready(&mut first).await;
    let mut attachment = kernel
        .open(create(first.cwd()), who(), OpenOptions::default())
        .await
        .unwrap();
    command(&mut attachment, 1, "/hello-board").await;
    fire(&mut attachment, 2, "hello.tick").await;
    let intent = intent(3);
    attachment.handle.submit(
        intent.clone(),
        Input::text("run the bar", Origin::surface("test")),
    );
    until_completed(&mut attachment).await;
    assert_eq!(rows(&board(&attachment))[0][2], "running");
    kernel.shutdown().await.unwrap();
    drop(attachment);

    let home = first.into_home();
    let cwd = home.path().to_path_buf();
    let mut second = hello(home, NOTHING);
    let kernel = ready(&mut second).await;
    let mut attachment = kernel
        .open(
            SessionSelector::Latest { cwd },
            who(),
            OpenOptions::default(),
        )
        .await
        .unwrap();

    assert_eq!(
        rows(&board(&attachment))[0][2],
        "running",
        "the board is what the last run left"
    );
    assert!(
        attachment.snapshot.signals.is_empty(),
        "and the bar is gone: {:?}",
        attachment.snapshot.signals
    );

    // A second tick moves the same row on, which it could not do if the
    // journal had handed this run a fresh board.
    fire(&mut attachment, 4, "hello.tick").await;
    assert_eq!(rows(&board(&attachment))[0][2], "done");

    kernel.shutdown().await.unwrap();
}

/// Fold frames until the scripted turn completes.
async fn until_completed(attachment: &mut Attachment) -> Vec<bingo_sdk::Frame> {
    let mut seen = Vec::new();
    let deadline = tokio::time::sleep(LIMIT + Duration::from_secs(10));
    tokio::pin!(deadline);
    loop {
        tokio::select! {
            frame = attachment.events.next() => {
                let frame = frame.expect("the stream stays open");
                attachment.snapshot.apply(&frame);
                let done = matches!(frame.event, Event::TurnCompleted { .. });
                seen.push(frame);
                if done {
                    return seen;
                }
            }
            _ = &mut deadline => panic!("the turn never completed"),
        }
    }
}

async fn applied(attachment: &mut Attachment, mut waiting: Vec<IntentId>) {
    tokio::time::timeout(LIMIT, async {
        while !waiting.is_empty() {
            let frame = attachment
                .events
                .next()
                .await
                .expect("the stream stays open");
            attachment.snapshot.apply(&frame);
            if let Event::IntentAck { intent, outcome } = frame.event
                && waiting.contains(&intent)
            {
                assert!(
                    matches!(outcome, IntentOutcome::Applied { .. }),
                    "{outcome:?}"
                );
                waiting.retain(|id| id != &intent);
            }
        }
    })
    .await
    .expect("both buttons finish");
}
