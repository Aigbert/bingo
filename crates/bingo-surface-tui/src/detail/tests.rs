use crate::test_support::ember::{self, Band, Capture};
use crate::test_support::*;
use crate::{clock::Now, ui::Ui};
use bingo_sdk::{Event, ItemBody, ItemStatus, SessionState};
use crossterm::event::{KeyCode, KeyEvent};

fn fixture() -> (SessionState, Ui) {
    let (mut state, _) = ember::story::scene("done");
    let mut summary = state.summary.clone();
    summary.model = Some("opus-5.5".into());
    summary.effort = Some(bingo_sdk::Effort::High);
    state.apply(&frame(state.seq.0 + 1, Event::SessionUpdated { summary }));
    let (mut ui, _) = scene();
    ui.catalogs
        .values
        .insert("tools".into(), ember::story::catalogue());
    (state, ui)
}

fn press(state: &SessionState, ui: &mut Ui, now: &mut Now, key: KeyEvent) {
    ui.keyed_at = now.instant;
    assert!(crate::input::on_key(ui, &solo(state), key, *now).is_empty());
    *now = later(*now, 60);
    drawn(100, 32, &solo(state), ui, *now);
}

fn setup(name: &str, state: &SessionState, ui: &mut Ui, now: &mut Now) {
    drawn(100, 32, &solo(state), ui, *now);
    if name.starts_with("ledger") {
        press(state, ui, now, ctrl('o'));
        let failed = state
            .transcript
            .items()
            .iter()
            .find(|item| item.status == ItemStatus::Failed)
            .unwrap();
        for _ in 0..state.transcript.items().len() {
            if ui.focus.detail() == Some(&failed.id) {
                break;
            }
            press(state, ui, now, key(KeyCode::Down));
        }
        assert_eq!(ui.focus.detail(), Some(&failed.id));
        press(state, ui, now, key(KeyCode::Enter));
    } else if name.starts_with("find") {
        press(state, ui, now, ctrl('f'));
        for c in "reset".chars() {
            press(state, ui, now, typed(c));
        }
    }
}

#[test]
fn m195_full_frames_follow_the_prototype_keys() {
    for name in [
        "done-detail",
        "done-well",
        "done-folded",
        "ledger",
        "ledger-up",
        "find",
        "find-up",
        "find-back",
    ] {
        let capture = Capture::load(name);
        capture.render(|| {
            let (state, mut ui) = fixture();
            let mut now = Capture::load("done").now();
            setup(name, &state, &mut ui, &mut now);
            // Scene setup precedes the capture's private clock and key sequence.
            now = Capture::load("done").now();
            for action in &capture.entry.after {
                if let Some(ms) = action.as_i64() {
                    now = later(now, ms);
                } else if let Some(chord) = action.as_str().filter(|s| !s.starts_with("shot:")) {
                    let event = match chord {
                        "ctrl+o" => ctrl('o'),
                        "up" => key(KeyCode::Up),
                        "down" => key(KeyCode::Down),
                        "enter" => key(KeyCode::Enter),
                        "esc" => key(KeyCode::Esc),
                        _ => panic!("{chord}"),
                    };
                    press(&state, &mut ui, &mut now, event);
                }
                drawn(100, 32, &solo(&state), &mut ui, now);
            }
            now.wall = capture.now().wall;
            let screen = drawn(100, 32, &solo(&state), &mut ui, now);
            ember::assert_regions(&capture, Band::Frame, screen.buffer(), &ui.shown.regions);
        });
    }
}

#[test]
fn m195_find_escape_restores_the_opening_scroll_and_enter_keeps_the_hit() {
    let (state, mut ui) = fixture();
    let mut now = still(Capture::load("done").now());
    drawn(100, 20, &solo(&state), &mut ui, now);
    ui.scroll = crate::scroll::Scroll::park(2, now.instant);
    drawn(100, 20, &solo(&state), &mut ui, now);
    let top = ui.shown.top;
    crate::input::on_key(&mut ui, &solo(&state), ctrl('f'), now);
    for c in "reset".chars() {
        crate::input::on_key(&mut ui, &solo(&state), typed(c), now);
    }
    drawn(100, 20, &solo(&state), &mut ui, now);
    assert_ne!(ui.shown.top, top);
    crate::input::on_key(&mut ui, &solo(&state), key(KeyCode::Esc), now);
    now = later(now, 200);
    drawn(100, 20, &solo(&state), &mut ui, now);
    assert_eq!(ui.shown.top, top);
    crate::input::on_key(&mut ui, &solo(&state), ctrl('f'), now);
    for c in "reset".chars() {
        crate::input::on_key(&mut ui, &solo(&state), typed(c), now);
    }
    drawn(100, 20, &solo(&state), &mut ui, now);
    let found = ui.shown.top;
    crate::input::on_key(&mut ui, &solo(&state), key(KeyCode::Enter), now);
    drawn(100, 20, &solo(&state), &mut ui, now);
    assert_eq!(ui.shown.top, found);
}

#[test]
fn m195_detail_esc_closes_without_answering_an_ask_or_stopping_a_turn() {
    let (mut state, mut ui) = fixture();
    let mut now = still(Capture::load("done").now());
    press(&state, &mut ui, &mut now, ctrl('o'));
    state.apply(&frame(
        state.seq.0 + 1,
        Event::InteractionOpened {
            interaction: permission(None, None),
        },
    ));
    press(&state, &mut ui, &mut now, key(KeyCode::Esc));
    assert!(matches!(ui.focus, crate::focus::Focus::Composer));
    assert!(ui.focus.detail().is_none());
    assert!(ui.wells.is_empty());
    assert!(ui.stop_asked.is_none());
    assert_eq!(state.interactions.len(), 1);
}

#[test]
fn m195_opening_detail_selects_the_first_row_of_the_newest_finished_turn() {
    let (state, mut ui) = fixture();
    let now = Capture::load("done").now();
    crate::input::open_detail(&mut ui, &state, now);
    let item = state.transcript.get(ui.focus.detail().unwrap()).unwrap();
    assert!(matches!(item.body, ItemBody::Reasoning { .. }));
    assert_eq!(item.turn.as_ref().unwrap().as_str(), "ember-1");
}

#[test]
fn clicking_a_step_keeps_detail_keys_and_dragging_its_well_keeps_the_output() {
    use bingo_sdk::{ItemId, ToolOutput};
    use crossterm::event::{KeyModifiers, MouseButton, MouseEvent, MouseEventKind};
    let state = folded(vec![frame(
        1,
        Event::ItemCompleted {
            item: tool(
                "read",
                "Inspect",
                serde_json::json!({"file_path":"note.txt"}),
                Some(ToolOutput::text("first line\nsecond line")),
                ItemStatus::Completed,
            ),
        },
    )]);
    let tree = solo(&state);
    let (mut ui, now) = scene();
    render(&state, &mut ui, now);
    crate::input::open_detail(&mut ui, &state, now);
    let screen = render(&state, &mut ui, now);
    let press = |ui: &mut Ui, kind, column, row| {
        crate::pointer::on_mouse(
            ui,
            &tree,
            MouseEvent {
                kind,
                column,
                row,
                modifiers: KeyModifiers::NONE,
            },
            now,
        )
    };
    let row = row_carrying(&screen, "note.txt");
    press(&mut ui, MouseEventKind::Down(MouseButton::Left), 13, row);
    press(&mut ui, MouseEventKind::Up(MouseButton::Left), 13, row);
    assert!(matches!(ui.focus, crate::focus::Focus::Detail(_)));
    let screen = render(&state, &mut ui, now);
    let row = row_carrying(&screen, "first line");
    press(&mut ui, MouseEventKind::Down(MouseButton::Left), 7, row);
    press(&mut ui, MouseEventKind::Drag(MouseButton::Left), 12, row);
    assert!(ui.wells.values().any(|id| id == &ItemId::from_raw("read")));
    let copied = press(&mut ui, MouseEventKind::Up(MouseButton::Left), 12, row);
    assert!(matches!(copied.as_slice(), [crate::effect::Effect::Copy(text)] if text == "first"));
}
#[test]
fn typing_in_detail_dispatches_the_letter_to_the_composer() {
    let (state, mut ui) = fixture();
    let now = Capture::load("done").now();
    crate::input::open_detail(&mut ui, &state, now);
    crate::input::on_key(&mut ui, &solo(&state), typed('x'), now);
    assert!(matches!(ui.focus, crate::focus::Focus::Composer));
    assert!(ui.focus.detail().is_none());
    assert_eq!(ui.composer.text(), "x");
}

#[test]
fn detail_closed_never_includes_a_loose_item_in_its_turn() {
    let state = folded(vec![frame(
        1,
        Event::ItemCompleted {
            item: loose(user("loose", "outside any turn")),
        },
    )]);
    let folds = Default::default();
    let pictures = Default::default();
    let linked = Default::default();
    let rows = crate::transcript::Rows::of(&state, 80, &folds, &pictures, &linked, scene().1);
    let roles = crate::turns::Roles::of(&Default::default(), &rows);
    let item = &state.transcript.items()[0];
    assert_eq!(roles.of_item(&item.id), &crate::turns::Role::Own);
    assert!(crate::transcript::band::revision(item, &rows).is_none());
}

#[test]
fn ledger_setup_opens_the_first_of_two_failed_steps() {
    let step = |id: &str, status| {
        let mut call = tool(
            id,
            "Execute",
            serde_json::json!({"command": id}),
            Some(bingo_sdk::ToolOutput::text(id)),
            status,
        );
        call.turn = Some(bingo_sdk::TurnId::from_raw("trn_1"));
        call
    };
    let state = folded(
        [
            step("passed", ItemStatus::Completed),
            step("first", ItemStatus::Failed),
            step("second", ItemStatus::Failed),
        ]
        .into_iter()
        .enumerate()
        .map(|(at, item)| frame(at as u64 + 1, Event::ItemCompleted { item }))
        .collect(),
    );
    let turn = bingo_sdk::TurnId::from_raw("trn_1");
    let failed: Vec<_> = state
        .transcript
        .items()
        .iter()
        .filter(|item| item.status == ItemStatus::Failed && item.turn.as_ref() == Some(&turn))
        .map(|item| item.id.as_str().to_owned())
        .collect();
    assert_eq!(failed, ["first", "second"], "both failures share the turn");
    let (mut ui, mut now) = scene();
    setup("ledger", &state, &mut ui, &mut now);
    let first = bingo_sdk::ItemId::from_raw("first");
    assert_eq!(ui.focus.detail(), Some(&first));
    assert_eq!(ui.wells.get(&turn), Some(&first));
}

#[test]
fn closing_detail_holds_the_drawn_place_when_more_rows_arrive() {
    for at_tail in [false, true] {
        let (mut state, mut ui) = fixture();
        let now = still(Capture::load("done").now());
        drawn(100, 20, &solo(&state), &mut ui, now);
        crate::input::open_detail(&mut ui, &state, now);
        drawn(100, 20, &solo(&state), &mut ui, now);
        if at_tail {
            let last = state
                .transcript
                .items()
                .iter()
                .rev()
                .find(|item| super::row(item))
                .unwrap();
            ui.focus = crate::focus::Focus::Detail(last.id.clone());
            ui.scroll.end(now.instant);
            drawn(100, 20, &solo(&state), &mut ui, now);
            assert_eq!(ui.shown.below(), 0);
        }
        crate::detail::close(&mut ui, &solo(&state), now);
        assert_eq!(ui.scroll.following(), at_tail);
        drawn(100, 20, &solo(&state), &mut ui, now);
        let top = ui.shown.top;
        for n in 0..10 {
            state.apply(&frame(
                state.seq.0 + 1,
                Event::ItemCompleted {
                    item: loose(user(&format!("later-{n}"), "more rows arrive")),
                },
            ));
        }
        drawn(100, 20, &solo(&state), &mut ui, now);
        if at_tail {
            assert_eq!(ui.shown.below(), 0);
        } else {
            assert_eq!(ui.shown.top, top);
        }
    }
}

#[test]
fn closing_detail_follows_when_the_folded_stream_fits() {
    let (state, mut ui) = fixture();
    let now = still(Capture::load("done").now());
    let fits = (10..80u16)
        .find(|&height| {
            let (_, mut probe) = fixture();
            drawn(100, height, &solo(&state), &mut probe, now);
            let folded = probe.shown.top == 0 && probe.shown.below() == 0;
            crate::input::open_detail(&mut probe, &state, now);
            drawn(100, height, &solo(&state), &mut probe, now);
            folded && probe.shown.height > probe.shown.regions.transcript.height as usize
        })
        .expect("a height where only the open detail overflows");
    drawn(100, fits, &solo(&state), &mut ui, now);
    crate::input::open_detail(&mut ui, &state, now);
    drawn(100, fits, &solo(&state), &mut ui, now);
    ui.scroll = crate::scroll::Scroll::Offset(ui.shown.height);
    drawn(100, fits, &solo(&state), &mut ui, now);
    assert!(ui.shown.below() > 0);
    crate::detail::close(&mut ui, &solo(&state), now);
    assert!(ui.scroll.following());
}
