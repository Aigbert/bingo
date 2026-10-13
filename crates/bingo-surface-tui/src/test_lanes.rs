//! The fixtures of M11d: the three lanes of ADR-0013 as frames, the demo
//! plugin's board as a value, and the pins a person makes. They are kept
//! beside [`crate::test_support`] and re-exported from it, so a test still
//! reaches every fixture through one import.

use bingo_sdk::{Event, SessionId, SessionState, View};
use serde_json::Value;

use crate::test_support::{extended, folded, frame, user};
use crate::ui::Ui;

/// A plugin publishing live state onto the stream: never journaled, gone on
/// a resume, replaced in place by the next one (ADR-0013 §2).
pub fn signalled(plugin: &str, kind: &str, payload: Value) -> Event {
    Event::Signal {
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
        child: Box::new(View::Stack {
            children: vec![
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
                            label: "Tick".into(),
                            action: bingo_sdk::Action {
                                name: "board.tick".into(),
                                args: Value::Null,
                            },
                            key: None,
                        },
                        bingo_sdk::ActionItem {
                            label: "Reset".into(),
                            action: bingo_sdk::Action {
                                name: "board.reset".into(),
                                args: Value::Null,
                            },
                            key: None,
                        },
                    ],
                },
            ],
        }),
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

/// One of the demo plugin's cards, by the kind it published under.
pub fn demo_card(kind: &str) -> crate::rail::CardId {
    crate::rail::CardId {
        plugin: "bingo.demo.ui".into(),
        kind: kind.into(),
    }
}

/// A person who pinned the board into the root session's rail.
pub fn pin_board(ui: &mut Ui) {
    ui.pinned.insert(crate::rail::Pin {
        session: SessionId::from_raw("ses_1"),
        card: demo_card("board"),
    });
}

#[test]
fn a_stranger_places_every_slot_without_a_surface_registration() {
    use bingo_sdk::{Action, ActionItem, ItemId, ItemStatus, Tone, ToolOutput, slot::{self, Slot}};
    use crate::{effect::Effect, input, panel, rail, test_support::*};
    let (mut ui, now) = scene();
    let payload = slot::with(serde_json::json!({}), [
        Slot::Status { text: "stranger status".into(), tone: Tone::Good, until: None },
        Slot::Activity { verb: Some("Inspecting".into()), view: View::Panel {
            title: "stranger activity".into(), child: Box::new(View::text("activity body")),
        } },
        Slot::Rail { title: "stranger rail".into(), view: View::text("rail body") },
        Slot::Panel { title: "stranger panel".into(), view: View::text("panel body") },
        Slot::Mode { current: "awake".into(), modes: ["rest", "awake"].map(|label| ActionItem {
            label: label.into(), action: Action { name: "stranger.choose".into(), args: label.into() }, key: None,
        }).to_vec() },
        Slot::Row { item: ItemId::from_raw("itm_slot"), mark: Some('!'), title: Some("stranger row".into()),
            badge: Some("badge".into()), body: Some(View::text("row body")), actions: vec![] },
    ]);
    let state = folded(vec![
        frame(1, Event::ItemCompleted { item: tool("itm_slot", "Unfamiliar", serde_json::json!({}),
            Some(ToolOutput::text("old body")), ItemStatus::Completed) }),
        frame(2, signalled("stranger", "placed", payload)),
    ]);
    let screen = draw_sized(120, 40, &state, &ui, now);
    for text in ["stranger status", "stranger activity", "activity body", "stranger rail", "rail body", "awake", "stranger row", "badge", "row body"] {
        assert!(screen.contains(text), "missing {text}:\n{screen}");
    }
    assert!(!screen.contains("old body"));
    assert_eq!(rail::cards(&state, &state.summary.id, &ui.pinned).len(), 1);
    let sheet = panel::lines(&state, &state.summary.id, 0, &ui.pinned, 80, 30, now);
    let sheet = sheet.iter().map(ToString::to_string).collect::<Vec<_>>().join("\n");
    assert!(sheet.contains("stranger panel") && sheet.contains("panel body"), "{sheet}");
    let effects = input::on_key(&mut ui, &solo(&state), key(crossterm::event::KeyCode::BackTab), now);
    assert!(matches!(&effects[..], [Effect::Submit(bingo_sdk::Input::Action { action })]
        if action.name == "stranger.choose" && action.args == "rest"));
}
