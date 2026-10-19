use super::*;
use crate::clock::Now;
use crate::test_support::*;
use bingo_sdk::{Event, ItemStatus, SessionState, TurnStatus, render::Component};
use serde_json::json;

fn with_rows<R>(state: &SessionState, now: Now, f: impl FnOnce(&Rows<'_>) -> R) -> R {
    let folds = Default::default();
    let pictures = Default::default();
    let linked = Default::default();
    f(&Rows::of(state, 80, &folds, &pictures, &linked, now))
}

#[test]
fn working_begins_at_five_hundred_ms_with_an_exact_deadline() {
    let state = folded(vec![frame(1, started("trn_1"))]);
    let now = scene().1;
    with_rows(&state, later(now, 499), |rows| {
        let drawn = draw(&state, rows);
        assert!(drawn.lines.is_empty());
        assert_eq!(drawn.next, Some(later(now, 500).instant));
        assert!(props(rows).is_none());
    });
    with_rows(&state, later(now, 500), |rows| {
        assert_eq!(
            serde_json::to_value(props(rows).unwrap()).unwrap(),
            json!({
                "component":"Spinner","props":{"word":"working","message":null,"suffix":"","mode":"requesting"}
            })
        );
        assert!(draw(&state, rows).lines[0].to_string().contains("working"));
    });
}

#[test]
fn an_open_thought_has_its_own_clock_and_exact_second_deadline() {
    let mut thought = item(
        "thought",
        ItemStatus::Running,
        ItemBody::Reasoning {
            text: "considering".into(),
            provider_metadata: Default::default(),
        },
    );
    thought.started_at = ts() + jiff::SignedDuration::from_millis(123);
    let state = folded(vec![
        frame(1, started("trn_1")),
        frame(2, Event::ItemStarted { item: thought }),
    ]);
    let now = still(scene().1);
    for (at, clock) in [(1122, false), (1123, true)] {
        with_rows(&state, later(now, at), |rows| {
            let Props::Spinner(props) = props(rows).unwrap() else {
                panic!("spinner");
            };
            assert_eq!((&*props.word, &*props.mode), ("thinking", "thinking"));
            let drawn = draw(&state, rows);
            assert_eq!(drawn.lines[0].to_string().contains("1s"), clock);
            assert_eq!(
                drawn.next,
                Some(later(now, if clock { 2123 } else { 1123 }).instant)
            );
            if clock {
                assert_eq!(drawn.lines[0].width(), 78);
            }
        });
    }
}

#[test]
fn a_pending_call_does_not_hold_back_the_placed_verb() {
    let state = folded(vec![
        frame(1, started("trn_1")),
        frame(
            2,
            Event::ItemStarted {
                item: tool("pending", "Inspect", json!({}), None, ItemStatus::Pending),
            },
        ),
        frame(
            3,
            extended(
                "test",
                "activity",
                bingo_sdk::slot::with(
                    json!({}),
                    [bingo_sdk::slot::Slot::Activity {
                        verb: Some("Checking the wording".into()),
                        view: View::text("").into(),
                    }],
                ),
            ),
        ),
    ]);
    with_rows(&state, later(scene().1, 600), |rows| {
        let Props::Spinner(props) = props(rows).unwrap() else {
            panic!("spinner");
        };
        assert_eq!(props.word, "Checking the wording");
        assert_eq!(props.mode, "requesting");
    });
}

#[test]
fn narration_followed_by_a_step_allows_working_but_an_answer_does_not() {
    let mut state = folded(vec![
        frame(1, started("trn_1")),
        frame(
            2,
            Event::ItemCompleted {
                item: assistant("words", "checking", ItemStatus::Completed),
            },
        ),
    ]);
    with_rows(&state, later(scene().1, 600), |rows| {
        assert!(draw(&state, rows).lines.is_empty())
    });
    state.apply(&frame(
        3,
        Event::ItemCompleted {
            item: tool("step", "Inspect", json!({}), None, ItemStatus::Completed),
        },
    ));
    with_rows(&state, later(scene().1, 600), |rows| {
        assert!(draw(&state, rows).lines[0].to_string().contains("working"))
    });
    state.apply(&frame(
        4,
        Event::ItemStarted {
            item: assistant("answer", "Here", ItemStatus::Running),
        },
    ));
    with_rows(&state, later(scene().1, 600), |rows| {
        assert!(draw(&state, rows).lines.is_empty())
    });
}

#[test]
fn escape_draws_stopping_on_its_own_frame_until_the_turn_completes() {
    for item in [
        running_tool("step", "Inspect", "checking"),
        assistant("answer", "Here", ItemStatus::Running),
    ] {
        let mut state = folded(vec![
            frame(1, started("trn_1")),
            frame(2, Event::ItemStarted { item }),
        ]);
        let (mut ui, now) = scene();
        let now = later(now, 100);
        draw_tree(80, 24, &solo(&state), &mut ui, now);
        crate::input::on_key(
            &mut ui,
            &solo(&state),
            crossterm::event::KeyCode::Esc.into(),
            now,
        );
        let drawn = draw_tree(80, 24, &solo(&state), &mut ui, now);
        assert!(drawn.contains("stopping"), "{drawn}");
        state.apply(&frame(
            3,
            completed(
                "trn_1",
                TurnStatus::Interrupted {
                    reason: bingo_sdk::InterruptReason::UserCancel,
                },
            ),
        ));
        assert!(!draw_tree(80, 24, &solo(&state), &mut ui, now).contains("stopping"));
    }
}

#[test]
fn a_mods_spinner_answer_redraws_the_stream_row() {
    let mut state = folded(vec![frame(1, started("trn_1"))]);
    state.config.kernel["render"] = json!({"Spinner":[{}]});
    let (mut ui, now) = scene();
    let now = later(now, 500);
    draw_tree(80, 24, &solo(&state), &mut ui, now);
    let key = ui
        .shown
        .pulls
        .iter()
        .find(|key| key.request.component == Component::Spinner)
        .unwrap()
        .clone();
    assert_eq!(
        key.request.props,
        json!({"word":"working","message":null,"suffix":"","mode":"requesting"})
    );
    ui.placed.pulled.request(&key);
    ui.placed
        .pulled
        .answer(key, View::text("Spinner from a mod"));
    assert!(draw_tree(80, 24, &solo(&state), &mut ui, now).contains("Spinner from a mod"));
}

#[test]
fn spinner_requests_follow_the_visible_window() {
    let mut state = long_transcript(50);
    state.apply(&frame(state.seq.0 + 1, started("live")));
    state.config.kernel["render"] = json!({"Spinner":[{}]});
    let (mut ui, born) = scene();
    let now = still(later(born, 10_000));
    let tree = solo(&state);
    draw_tree(80, 24, &tree, &mut ui, now);
    assert!(
        ui.shown
            .pulls
            .iter()
            .any(|key| key.request.component == Component::Spinner)
    );
    ui.scroll.home(
        ui.shown.height,
        ui.shown.regions.transcript.height as usize,
        now.instant - std::time::Duration::from_secs(1),
    );
    draw_tree(80, 24, &tree, &mut ui, now);
    assert!(
        !ui.shown
            .pulls
            .iter()
            .any(|key| key.request.component == Component::Spinner)
    );
}

#[test]
fn the_live_rows_appearance_deadline_follows_its_visible_window() {
    let (mut ui, born) = scene();
    let mut state = long_transcript(50);
    let mut start = frame(state.seq.0 + 1, started("live"));
    start.ts = later(born, 10_000).wall;
    state.apply(&start);
    let now = still(later(born, 10_499));
    let tree = solo(&state);
    draw_tree(80, 24, &tree, &mut ui, now);
    assert_eq!(ui.shown.deadline(), Some(later(born, 10_500).instant));
    assert!(!draw_tree(80, 24, &tree, &mut ui, now).contains("working"));
    ui.scroll.home(
        ui.shown.height,
        ui.shown.regions.transcript.height as usize,
        now.instant - std::time::Duration::from_secs(1),
    );
    draw_tree(80, 24, &tree, &mut ui, now);
    assert_eq!(ui.shown.deadline(), None);
    ui.scroll = Default::default();
    assert!(draw_tree(80, 24, &tree, &mut ui, still(later(born, 10_500))).contains("working"));
}
