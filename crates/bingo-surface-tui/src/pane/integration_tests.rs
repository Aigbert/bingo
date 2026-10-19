use crate::test_support::*;
use bingo_sdk::slot::Slot;
use serde_json::json;

fn state_with(place: serde_json::Value) -> bingo_sdk::SessionState {
    let mut state = state();
    state.config.kernel["render"] = json!({"Pane":[{}]});
    let slot: Slot = serde_json::from_value(json!({"kind":"pane","id":"counter","title":"Counter",
        "focus":true,"closeOnEscape":true,"holdToasts":false,"asked":true,
        "place":place,"size":{"rows":{"min":8,"ideal":10,"max":20},
        "columns":{"min":48,"ideal":64,"max":80}},
        "view":{"request":{"component":"Pane","requestId":"counter","surface":"terminal",
        "props":{},"viewport":{"columns":0,"rows":0}}}}))
    .unwrap();
    state.signals.entry("counter".into()).or_default().insert(
        "pane".into(),
        bingo_sdk::slot::with(json!({}), [slot]).into(),
    );
    state
}
#[test]
fn seated_pulls_change_place_width_and_height_and_collapse_without_a_request() {
    let state = state_with(json!(["side", "above"]));
    let (mut ui, now) = scene();
    for (width, height, place, columns, rows) in
        [(200, 50, "side", 64, 45), (100, 30, "above", 100, 9)]
    {
        draw_sized(width, height, &state, &mut ui, now);
        let viewport = serde_json::to_value(&ui.shown.pulls[0].request.viewport).unwrap();
        assert_eq!(
            viewport,
            json!({"place":place,"columns":columns,"rows":rows})
        );
    }
    let screen = draw_sized(60, 15, &state, &mut ui, now);
    assert!(ui.shown.pulls.is_empty());
    assert!(screen.contains("Counter ▸"), "{screen}");
}

fn with_text(mut state: bingo_sdk::SessionState) -> bingo_sdk::SessionState {
    state
        .signals
        .get_mut("counter")
        .unwrap()
        .get_mut("pane")
        .unwrap()["slots"][0]["view"] =
        json!({"kind":"Text","children":["Pane content\nSecond row\nThird row"]});
    state
}
#[test]
fn pane_screens_cover_docked_collapsed_and_floating_places() {
    let state = with_text(state_with(json!(["side", "above"])));
    for (name, width, height) in [
        ("pane_side", 200, 50),
        ("pane_above", 100, 30),
        ("pane_collapsed", 60, 15),
    ] {
        let (mut ui, now) = scene();
        let screen = draw_sized(width, height, &state, &mut ui, now);
        insta::assert_snapshot!(name, screen);
    }
    let mut state = with_text(state_with(
        json!([{"rect":{"anchor":"bottomRight","width":"40%","height":"30%"}}]),
    ));
    let (mut ui, now) = scene();
    let screen = draw_sized(120, 40, &state, &mut ui, now);
    assert!(screen.contains("Pane content"), "{screen}");
    insta::assert_snapshot!("pane_corner", screen);
    state.apply(&frame(1, opened(permission(Some("Edit(src/)"), None))));
    ui.dialog.focus_on(state.interactions.first());
    let screen = draw_sized(120, 40, &state, &mut ui, still(now));
    assert!(screen.contains("No, and tell it why"), "{screen}");
    insta::assert_snapshot!("permission_over_pane", screen);
}
fn mouse(
    kind: crossterm::event::MouseEventKind,
    column: u16,
    row: u16,
) -> crossterm::event::MouseEvent {
    crossterm::event::MouseEvent {
        kind,
        column,
        row,
        modifiers: crossterm::event::KeyModifiers::NONE,
    }
}
#[test]
fn floating_pointer_events_reach_the_topmost_pane_and_never_select_the_transcript() {
    use crossterm::event::{MouseButton::Left, MouseEventKind::*};
    let mut state = with_text(state_with(json!(["overlay"])));
    let mut second = state.signals["counter"]["pane"]["slots"][0].clone();
    second["id"] = json!("newest");
    second["title"] = json!("Newest");
    second["view"] = json!({"kind":"Text","children":[(0..70).map(|n|format!("new row {n}\n")).collect::<String>()]});
    state
        .signals
        .get_mut("counter")
        .unwrap()
        .get_mut("pane")
        .unwrap()["slots"]
        .as_array_mut()
        .unwrap()
        .push(second);
    let tree = solo(&state);
    let (mut ui, now) = scene();
    draw_tree(120, 40, &tree, &mut ui, now);
    let area = ui.shown.seating.panes.last().unwrap().area;
    let top = ui.shown.top;
    crate::pointer::on_mouse(
        &mut ui,
        &tree,
        mouse(ScrollDown, area.x + 1, area.y + 2),
        now,
    );
    assert_eq!(ui.shown.top, top);
    assert_eq!(
        ui.placed.panes.scroll.values().copied().collect::<Vec<_>>(),
        vec![3]
    );
    crate::pointer::on_mouse(&mut ui, &tree, mouse(Down(Left), area.x, area.y + 1), now);
    assert!(matches!(ui.focus, crate::focus::Focus::Composer));
    assert!(
        ui.placed
            .panes
            .selection
            .as_ref()
            .unwrap()
            .0
            .ends_with("/newest")
    );
    crate::pointer::on_mouse(
        &mut ui,
        &tree,
        mouse(Drag(Left), area.x + 7, area.y + 1),
        now,
    );
    assert!(ui.focus.selection().is_none());
    let effects =
        crate::pointer::on_mouse(&mut ui, &tree, mouse(Up(Left), area.x + 7, area.y + 1), now);
    assert!(
        matches!(&effects[..],[crate::effect::Effect::Copy(text)] if text=="new row"),
        "{effects:?}"
    );
}
#[test]
fn collapse_click_and_title_and_key_cycle_fit_without_changing_session_data() {
    use bingo_sdk::slot::Place;
    use crossterm::event::{
        KeyCode, KeyEvent, KeyModifiers, MouseButton::Left, MouseEventKind::Down,
    };
    let mut state = with_text(state_with(json!(["side", "above"])));
    state
        .signals
        .get_mut("counter")
        .unwrap()
        .get_mut("pane")
        .unwrap()["slots"][0]["view"] =
        json!({"kind":"Button","label":"Run","onPress":{"name":"counter:run","args":{}}});
    let tree = solo(&state);
    let (mut ui, now) = scene();
    draw_tree(60, 15, &tree, &mut ui, now);
    let target = ui.shown.waiting[0].1;
    crate::pointer::on_mouse(&mut ui, &tree, mouse(Down(Left), target.x, target.y), now);
    draw_tree(60, 15, &tree, &mut ui, now);
    assert_eq!(ui.shown.seating.panes[0].place, Place::Overlay);
    draw_tree(200, 50, &tree, &mut ui, now);
    super::key(
        &mut ui,
        &state,
        KeyEvent::new(KeyCode::Char('p'), KeyModifiers::CONTROL),
    );
    draw_tree(200, 50, &tree, &mut ui, now);
    assert_eq!(ui.shown.seating.panes[0].place, Place::Overlay);
    super::key(
        &mut ui,
        &state,
        KeyEvent::new(KeyCode::Char('p'), KeyModifiers::ALT),
    );
    draw_tree(200, 50, &tree, &mut ui, now);
    assert_eq!(ui.shown.seating.panes[0].place, Place::Above);
    let area = ui.shown.seating.panes[0].area;
    crate::pointer::on_mouse(
        &mut ui,
        &tree,
        mouse(Down(Left), area.right() - 1, area.y),
        now,
    );
    draw_tree(200, 50, &tree, &mut ui, now);
    assert_eq!(ui.shown.seating.panes[0].place, Place::Side);
    assert_eq!(tree.viewed().signals, state.signals);
}

#[test]
fn escape_closes_a_focused_pane_despite_a_retained_or_late_answer() {
    use crate::{effect::Effect, focus::Focus};
    use bingo_sdk::{Event, Input, View};
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
    let mut state = state_with(json!(["above"]));
    let (mut ui, now) = scene();
    ui.sync_panes(&state);
    draw_sized(80, 24, &state, &mut ui, now);
    let request = ui.shown.pulls[0].clone();
    ui.placed.pulled.request(&request);
    ui.placed
        .pulled
        .answer(request.clone(), View::text("Count: 1"));
    assert!(draw_sized(80, 24, &state, &mut ui, now).contains("Count: 1"));
    let effects = crate::input::on_key(
        &mut ui,
        &solo(&state),
        KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE),
        now,
    );
    assert!(
        matches!(&effects[..], [Effect::Submit(Input::Action { action, .. })]
        if action.name == "counter:ui" && action.args == json!({"handler":"counter","event":"close"}))
    );
    let closing = draw_sized(80, 24, &state, &mut ui, now);
    assert!(
        closing.contains("Count: 1"),
        "a pending close must not look closed before its slot is removed: {closing}"
    );
    assert_eq!(ui.shown.pulls, vec![request.clone()]);
    state.apply(&frame(
        1,
        Event::Signal {
            generation: None,
            plugin: "counter".into(),
            kind: "pane".into(),
            payload: serde_json::Value::Null,
        },
    ));
    ui.sync_panes(&state);
    assert!(matches!(ui.focus, Focus::Composer));
    ui.placed.pulled.retain(&[]);
    ui.placed
        .pulled
        .answer(request, View::text("late Count: 1"));
    let screen = draw_sized(80, 24, &state, &mut ui, now);
    assert!(!screen.contains("Count: 1"), "{screen}");
    assert!(!screen.contains("Counter"), "{screen}");
    assert!(ui.shown.seating.panes.is_empty());
    assert!(ui.shown.pulls.is_empty());
}

#[test]
fn revalidating_pane_keeps_art_and_clips_to_the_current_seat() {
    let mut state =
        state_with(json!([{"rect":{"anchor":"bottomRight","width":"40%","height":"40%"}}]));
    let (mut ui, now) = scene();
    state
        .signals
        .get_mut("counter")
        .unwrap()
        .get_mut("pane")
        .unwrap()["slots"][0]["size"] =
        json!({"rows":{"min":2,"ideal":8,"max":20},"columns":{"min":8,"ideal":22,"max":80}});
    let first_screen = draw_sized(120, 40, &state, &mut ui, now);
    assert!(first_screen.contains(crate::theme::ellipsis()));
    let first = ui.shown.pulls[0].clone();
    ui.placed.pulled.request(&first);
    ui.placed
        .pulled
        .answer(first.clone(), bingo_sdk::View::text("OLD ART".repeat(20)));
    assert!(draw_sized(120, 40, &state, &mut ui, now).contains("OLD ART"));
    state.invalidated.insert("counter".into(), 1);
    draw_sized(80, 24, &state, &mut ui, now);
    let next = ui.shown.pulls[0].clone();
    ui.placed.pulled.retain(std::slice::from_ref(&next));
    ui.placed.pulled.request(&next);
    ui.placed
        .pulled
        .answer(first, bingo_sdk::View::text("LATE ART"));
    let screen = draw_sized(80, 24, &state, &mut ui, now);
    assert!(screen.contains("OLD ART"), "{screen}");
    assert!(!screen.contains(crate::theme::ellipsis()), "{screen}");
    let area = ui.shown.seating.panes[0].area;
    for row in screen
        .lines()
        .skip(usize::from(area.y + 1))
        .take(usize::from(area.height - 1))
    {
        assert!(
            row.trim_matches('"')
                .chars()
                .skip(usize::from(area.right()))
                .all(|c| c == ' ')
        );
    }
    ui.placed
        .pulled
        .answer(next, bingo_sdk::View::text("NEW ART"));
    assert!(draw_sized(80, 24, &state, &mut ui, now).contains("NEW ART"));
}

#[test]
fn escape_keeps_an_ordinary_panes_close_behavior_during_a_turn() {
    use crate::effect::Effect;
    use bingo_sdk::Input;
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
    let mut state = state_with(json!(["above"]));
    state.apply(&frame(1, started("running")));
    let (mut ui, now) = scene();
    ui.sync_panes(&state);
    let effects = crate::input::on_key(
        &mut ui,
        &solo(&state),
        KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE),
        now,
    );
    assert!(
        matches!(&effects[..], [Effect::Submit(Input::Action { action, .. })] if action.args["event"] == "close")
    );
}

#[test]
fn side_then_dock_panes_remain_visible_at_both_widths() {
    let state = with_text(state_with(json!(["side", "dock"])));
    for (width, expected) in [
        (200, bingo_sdk::slot::Place::Side),
        (80, bingo_sdk::slot::Place::Dock),
    ] {
        let (mut ui, now) = scene();
        ui.sync_panes(&state);
        let screen = draw_sized(width, 24, &state, &mut ui, still(now));
        assert!(screen.contains("Pane content"), "{screen}");
        assert_eq!(ui.shown.seating.panes.len(), 1);
        assert_eq!(ui.shown.seating.panes[0].place, expected);
    }
}
