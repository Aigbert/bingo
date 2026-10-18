//! The fixtures of M11d: the three lanes of ADR-0013 as frames, the demo
//! plugin's board as a value, and the pins a person makes. They are kept
//! beside [`crate::test_support`] and re-exported from it, so a test still
//! reaches every fixture through one import.

use bingo_sdk::{Event, SessionState, View};
use serde_json::Value;

use crate::test_support::{extended, folded, frame, user};

/// A plugin publishing live state onto the stream: never journaled, gone on
/// a resume, replaced in place by the next one (ADR-0013 §2).
pub fn signalled(plugin: &str, kind: &str, payload: Value) -> Event {
    Event::Signal {
        generation: None,
        plugin: plugin.into(),
        kind: kind.into(),
        payload,
    }
}

/// One node as a payload: what a plugin puts on the wire when it describes a
/// screen rather than a record.
pub fn as_payload(view: View) -> Value {
    serde_json::to_value(view).expect("a view is serialisable")
}

pub fn progress_view(value: u64, total: u64) -> Value {
    as_payload(View::Progress {
        value,
        total: Some(total),
        label: Some("cargo test".into()),
    })
}

/// The board the demo plugin publishes: a table with buttons under it, which
/// is the shape every interactive panel has.
pub fn board_view() -> View {
    View::Panel {
        title: "Board".into(),
        child: Box::new(View::column(vec![
            View::Table {
                headers: vec!["id".into(), "state".into()],
                rows: vec![
                    vec!["1".into(), "ready".into()],
                    vec!["2".into(), "held".into()],
                ],
            },
            View::Actions {
                items: vec![
                    bingo_sdk::ActionItem {
                        meaning: None,
                        label: "Tick".into(),
                        action: bingo_sdk::Action {
                            name: "board.tick".into(),
                            args: Value::Null,
                        },
                        key: None,
                    },
                    bingo_sdk::ActionItem {
                        meaning: None,
                        label: "Reset".into(),
                        action: bingo_sdk::Action {
                            name: "board.reset".into(),
                            args: Value::Null,
                        },
                        key: None,
                    },
                ],
            },
        ])),
    }
}

/// A session the demo plugin has written to in two lanes at once: a board it
/// journaled, and the progress it is publishing as it goes.
pub fn boarded() -> SessionState {
    folded(vec![
        frame(
            1,
            Event::ItemCompleted {
                item: user("itm_1", "run the board"),
            },
        ),
        frame(
            2,
            extended("bingo.demo.ui", "board", as_payload(board_view())),
        ),
        frame(
            3,
            signalled("bingo.demo.ui", "progress", progress_view(24, 30)),
        ),
    ])
}

fn stranger_payload() -> Value {
    use bingo_sdk::{
        Action, ActionItem, ItemId, Tone,
        slot::{self, Slot},
    };
    slot::with(
        serde_json::json!({}),
        [
            Slot::Status {
                text: "stranger status".into(),
                tone: Tone::Good,
                until: None,
            },
            Slot::Activity {
                verb: Some("Inspecting".into()),
                view: View::Panel {
                    title: "stranger activity".into(),
                    child: Box::new(View::text("activity body")),
                }
                .into(),
            },
            Slot::Rail {
                title: "stranger rail".into(),
                view: View::text("rail body").into(),
            },
            Slot::Panel {
                title: "stranger panel".into(),
                view: View::text("panel body").into(),
            },
            Slot::Mode {
                current: "awake".into(),
                modes: ["rest", "awake"]
                    .map(|label| ActionItem {
                        meaning: None,
                        label: label.into(),
                        action: Action {
                            name: "stranger.choose".into(),
                            args: label.into(),
                        },
                        key: None,
                    })
                    .to_vec(),
            },
            Slot::Row {
                caption: None,
                item: ItemId::from_raw("itm_slot"),
                mark: Some('!'),
                title: Some("stranger row".into()),
                badge: Some("badge".into()),
                body: Some(View::text("row body")),
                actions: vec![],
            },
        ],
    )
}

#[test]
fn a_stranger_places_every_slot_without_a_surface_registration() {
    use crate::{effect::Effect, input, test_support::*};
    use bingo_sdk::{ItemStatus, ToolOutput};
    let (mut ui, now) = scene();
    let payload = stranger_payload();
    let state = folded(vec![
        frame(
            1,
            Event::ItemCompleted {
                item: tool(
                    "itm_slot",
                    "Unfamiliar",
                    serde_json::json!({}),
                    Some(ToolOutput::text("old body")),
                    ItemStatus::Completed,
                ),
            },
        ),
        frame(2, signalled("stranger", "placed", payload)),
    ]);
    let screen = draw_sized(120, 40, &state, &mut ui, now);
    for text in [
        "stranger status",
        "stranger activity",
        "activity body",
        "awake",
        "stranger row",
        "badge",
        "row body",
    ] {
        assert!(screen.contains(text), "missing {text}:\n{screen}");
    }
    assert!(!screen.contains("old body"));
    let effects = input::on_key(
        &mut ui,
        &solo(&state),
        key(crossterm::event::KeyCode::BackTab),
        now,
    );
    assert!(
        matches!(&effects[..], [Effect::CycleMode(bingo_sdk::Input::Action { action, .. })]
        if action.name == "stranger.choose" && action.args == "rest")
    );
}

/// The producer payload used by legacy task scenes; production never reads a task record.
pub fn task_payload(records: Value) -> Value {
    use bingo_sdk::{
        Tone, TreeNode,
        slot::{self, Slot},
    };
    let tasks = records.as_array().expect("task scene");
    let count = |status| tasks.iter().filter(|task| task["status"] == status).count();
    let title = format!(
        "{} {} ({} done, {} in progress, {} open)",
        tasks.len(),
        if tasks.len() == 1 { "task" } else { "tasks" },
        count("completed"),
        count("in_progress"),
        count("pending")
    );
    let shown: Vec<_> = tasks
        .iter()
        .filter(|task| tasks.len() <= 5 || task["status"] != "completed")
        .take(5)
        .collect();
    let tree = View::Tree {
        nodes: shown
            .iter()
            .map(|task| TreeNode {
                label: task["subject"].as_str().unwrap().into(),
                badge: task["owner"].as_str().map(str::to_owned),
                tone: match task["status"].as_str().unwrap() {
                    "completed" => Tone::Good,
                    "in_progress" => Tone::Attention,
                    _ => Tone::Neutral,
                },
                children: vec![],
            })
            .collect(),
    };
    let hidden = tasks.len() - shown.len();
    let child = if hidden == 0 {
        tree
    } else {
        let done = count("completed");
        let mut words = vec![];
        if hidden > done {
            words.push(format!("+{} pending", hidden - done));
        }
        if done > 0 {
            words.push(format!("{done} completed"));
        }
        View::column(vec![tree, View::text(format!("… {}", words.join(", ")))])
    };
    let verb = tasks
        .iter()
        .find(|task| task["status"] == "in_progress")
        .map(|task| {
            task["activeForm"]
                .as_str()
                .or(task["subject"].as_str())
                .unwrap()
                .to_string()
        });
    let slots = if tasks.is_empty() {
        vec![]
    } else {
        vec![Slot::Activity {
            verb,
            view: View::Panel {
                title,
                child: Box::new(child),
            }
            .into(),
        }]
    };
    slot::with(serde_json::json!({"tasks": records}), slots)
}

pub fn shell_payload(table: Value, item: &str) -> Value {
    use bingo_sdk::{
        ItemId, Tone,
        slot::{self, Slot},
    };
    slot::with(
        table,
        [
            Slot::Status {
                text: "1 shell".into(),
                tone: Tone::Neutral,
                until: None,
            },
            Slot::Row {
                caption: None,
                item: ItemId::from_raw(item),
                mark: None,
                title: None,
                badge: None,
                body: Some(View::text("Running in the background")),
                actions: vec![],
            },
        ],
    )
}

#[test]
fn quiet_is_a_record_flag_and_still_shows_displays_and_failures() {
    use crate::test_support::*;
    use bingo_sdk::{ItemBody, ItemStatus, ToolOutput};
    let (mut ui, now) = scene();
    let mut item = tool(
        "itm_quiet",
        "Unfamiliar",
        serde_json::json!({}),
        Some(ToolOutput {
            parts: vec![],
            is_error: false,
            display: Some(View::text("placed result")),
            upshot: None,
        }),
        ItemStatus::Completed,
    );
    let ItemBody::ToolCall { quiet, .. } = &mut item.body else {
        panic!("call")
    };
    *quiet = true;
    let state = folded(vec![frame(1, Event::ItemCompleted { item: item.clone() })]);
    let screen = render(&state, &mut ui, now).to_lowercase();
    assert!(screen.contains("placed result") && !screen.contains("unfamiliar"));
    item.status = ItemStatus::Failed;
    let state = folded(vec![frame(1, Event::ItemCompleted { item })]);
    let failed = render(&state, &mut ui, now);
    assert!(
        failed.contains("unfamiliar") && failed.contains("failed"),
        "a step of its turn: {failed}"
    );
}

#[test]
fn a_row_with_no_body_keeps_the_item_and_a_wide_mark_keeps_the_bullet() {
    use crate::test_support::*;
    use bingo_sdk::{
        ItemId, ItemStatus,
        slot::{self, Slot},
    };
    let (mut ui, now) = scene();
    let state = folded(vec![
        frame(
            1,
            Event::ItemCompleted {
                item: assistant("itm_row", "original words", ItemStatus::Completed),
            },
        ),
        frame(
            2,
            signalled(
                "stranger",
                "row",
                slot::with(
                    serde_json::json!({}),
                    [Slot::Row {
                        caption: None,
                        item: ItemId::from_raw("itm_row"),
                        mark: Some('界'),
                        title: None,
                        badge: Some("badge".into()),
                        body: None,
                        actions: vec![],
                    }],
                ),
            ),
        ),
    ]);
    let screen = render(&state, &mut ui, now);
    assert!(
        screen.contains("• original words") && screen.contains("badge"),
        "{screen}"
    );
    assert!(!screen.contains('界'));
}

/// The kernel's projection of one plugin's per-session setting (ADR-0009 §5).
pub fn plugin_view(plugin: &str, value: Value) -> Event {
    let value = if plugin == "bingo.permissions" {
        let current = value["mode"].as_str().unwrap_or("default").to_string();
        let modes = value["modes"]
            .as_array()
            .into_iter()
            .flatten()
            .map(|mode| bingo_sdk::ActionItem {
                meaning: Some(
                    match mode.as_str().unwrap() {
                        "default" => "trusted read-only tools run; everything else asks",
                        "acceptEdits" => {
                            "edits inside the working directories run without a prompt"
                        }
                        "plan" => "nothing that is not read-only runs at all",
                        "bypassPermissions" => {
                            "everything runs except what only a person may decide"
                        }
                        "dontAsk" => {
                            "nobody is there to answer, so what would have asked is denied"
                        }
                        _ => "",
                    }
                    .into(),
                ),
                label: match mode.as_str().unwrap() {
                    "acceptEdits" => "accept edits",
                    "bypassPermissions" => "bypass",
                    "dontAsk" => "don't ask",
                    other => other,
                }
                .into(),
                key: None,
                action: bingo_sdk::Action {
                    name: "permission".into(),
                    args: mode.clone(),
                },
            })
            .collect();
        bingo_sdk::slot::with(value, [bingo_sdk::slot::Slot::Mode { current, modes }])
    } else {
        value
    };
    let mut kernel = Value::Null;
    if plugin == "bingo.permissions" {
        kernel[bingo_sdk::ConfigView::POLICY] = plugin.into();
    }
    Event::ConfigChanged {
        config: bingo_sdk::ConfigView {
            kernel,
            plugins: std::collections::BTreeMap::from([(plugin.to_string(), value)]),
        },
    }
}
