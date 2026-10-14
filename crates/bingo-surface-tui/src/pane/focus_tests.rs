use crate::{focus::Focus, test_support::*};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers, MouseButton, MouseEvent, MouseEventKind};
use serde_json::json;

fn fixture(focus: bool, asked: bool, button: bool) -> bingo_sdk::SessionState {
    let mut state = state();
    let view = if button {
        json!({"kind":"Button","label":"Run","onPress":{"name":"test:run","args":{}}})
    } else {
        json!({"kind":"Box","children":[{"kind":"Text","children":["PANE ART"]}]})
    };
    state.signals.entry("test".into()).or_default().insert(
        "pane".into(),
        json!({"slots":[{
            "kind":"pane","id":"art","title":"Art","focus":focus,"asked":asked,
            "closeOnEscape":true,"holdToasts":false,"place":["above"],
            "size":{"rows":{"min":2,"ideal":4,"max":4},"columns":{"min":8,"ideal":22,"max":"100%"}},
            "view":view
        }]})
        .into(),
    );
    state
}
fn click(
    ui: &mut crate::ui::Ui,
    tree: &crate::tree::Tree,
    area: ratatui::layout::Rect,
    now: crate::clock::Now,
) {
    crate::pointer::on_mouse(
        ui,
        tree,
        MouseEvent {
            kind: MouseEventKind::Down(MouseButton::Left),
            column: area.x,
            row: area.y,
            modifiers: KeyModifiers::NONE,
        },
        now,
    );
}
fn type_x(ui: &mut crate::ui::Ui, tree: &crate::tree::Tree, now: crate::clock::Now) {
    crate::input::on_key(
        ui,
        tree,
        KeyEvent::new(KeyCode::Char('x'), KeyModifiers::NONE),
        now,
    );
    assert!(matches!(ui.focus, Focus::Composer));
    assert_eq!(ui.composer.text(), "x");
}
#[test]
fn waiting_click_opens_without_taking_composer_keys() {
    for (focus, button) in [(false, false), (false, true), (true, false), (true, true)] {
        let tree = solo(&fixture(focus, false, button));
        let (mut ui, now) = scene();
        draw_tree(80, 24, &tree, &mut ui, now);
        let area = ui.shown.waiting[0].1;
        click(&mut ui, &tree, area, now);
        assert!(matches!(ui.focus, Focus::Composer));
        type_x(&mut ui, &tree, now);
    }
}
#[test]
fn text_only_pane_click_leaves_keys_with_the_composer() {
    for focus in [false, true] {
        let tree = solo(&fixture(focus, true, false));
        let (mut ui, now) = scene();
        let screen = draw_tree(80, 24, &tree, &mut ui, now);
        assert!(screen.contains("PANE ART"), "{screen}");
        ui.focus = Focus::Composer;
        let area = ui.shown.seating.panes[0].area;
        click(&mut ui, &tree, area, now);
        assert!(matches!(ui.focus, Focus::Composer));
        type_x(&mut ui, &tree, now);
    }
}
/// `focus` asks for the keys when the pane opens; a person's click on a pane
/// with controls gives them whatever the pane asked for at open.
#[test]
fn clicked_pane_with_a_hotkey_takes_its_letter() {
    let mut state = fixture(false, true, true);
    state
        .signals
        .get_mut("test")
        .unwrap()
        .get_mut("pane")
        .unwrap()["slots"][0]["view"]["hotkey"] = json!("a");
    let tree = solo(&state);
    let (mut ui, now) = scene();
    draw_tree(80, 24, &tree, &mut ui, now);
    assert!(matches!(ui.focus, Focus::Composer));
    let area = ui.shown.seating.panes[0].area;
    click(&mut ui, &tree, area, now);
    assert!(matches!(ui.focus, Focus::Pane(_)));
    let effects = crate::input::on_key(
        &mut ui,
        &tree,
        KeyEvent::new(KeyCode::Char('a'), KeyModifiers::NONE),
        now,
    );
    assert!(
        matches!(&effects[..], [crate::effect::Effect::Submit(bingo_sdk::Input::Action { action })] if action.name == "test:run"),
        "{effects:?}"
    );
    assert_eq!(ui.composer.text(), "");
}
#[test]
fn composer_status_and_foot_clicks_release_a_button_pane() {
    let tree = solo(&fixture(true, true, true));
    let (mut ui, now) = scene();
    draw_tree(80, 24, &tree, &mut ui, now);
    let pane = ui.shown.seating.panes[0].area;
    let foot = ratatui::layout::Rect::new(0, 0, 5, 1);
    ui.shown.foot = Some(foot);
    for area in [ui.shown.regions.composer, ui.shown.regions.status, foot] {
        click(&mut ui, &tree, pane, now);
        assert!(matches!(ui.focus, Focus::Pane(_)));
        click(&mut ui, &tree, area, now);
        type_x(&mut ui, &tree, now);
        ui.composer.set("");
    }
}
#[test]
fn legacy_text_only_focus_forwards_a_character_and_enter() {
    let tree = solo(&fixture(true, true, false));
    let (mut ui, now) = scene();
    draw_tree(80, 24, &tree, &mut ui, now);
    let key = ui.shown.seating.panes[0].key.clone();
    ui.focus = Focus::Pane(key.clone());
    type_x(&mut ui, &tree, now);
    ui.focus = Focus::Pane(key);
    let effects = crate::input::on_key(
        &mut ui,
        &tree,
        KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE),
        now,
    );
    assert!(matches!(ui.focus, Focus::Composer));
    assert!(
        effects
            .iter()
            .any(|effect| matches!(effect, crate::effect::Effect::Submit(_))),
        "{effects:?}"
    );
}
#[test]
fn escape_keeps_close_or_release_semantics_for_text_only_focus() {
    for close in [false, true] {
        let mut state = fixture(true, true, false);
        state
            .signals
            .get_mut("test")
            .unwrap()
            .get_mut("pane")
            .unwrap()["slots"][0]["closeOnEscape"] = json!(close);
        let tree = solo(&state);
        let (mut ui, now) = scene();
        draw_tree(80, 24, &tree, &mut ui, now);
        ui.focus = Focus::Pane(ui.shown.seating.panes[0].key.clone());
        let effects = crate::input::on_key(
            &mut ui,
            &tree,
            KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE),
            now,
        );
        if close {
            assert!(
                matches!(&effects[..], [crate::effect::Effect::Submit(bingo_sdk::Input::Action { action })] if action.args["event"] == "close")
            );
        } else {
            assert!(matches!(ui.focus, Focus::Composer));
            assert!(effects.is_empty());
        }
    }
}
