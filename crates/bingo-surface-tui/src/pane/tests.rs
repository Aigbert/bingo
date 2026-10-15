use crate::test_support::*;
use bingo_sdk::{
    View,
    slot::{Shown, Slot},
};
fn pane(asked: bool) -> Slot {
    Slot::Pane {
        id: "counter".into(),
        title: "Counter pane".into(),
        focus: true,
        close_on_escape: true,
        hold_toasts: false,
        asked,
        place: bingo_sdk::slot::above(),
        size: bingo_sdk::slot::PaneSize::from_rows(20),
        view: Shown::View(View::text("Count: 0")),
    }
}
fn with(slot: Slot) -> bingo_sdk::SessionState {
    let mut state = state();
    state.signals.entry("counter".into()).or_default().insert(
        "pane".into(),
        bingo_sdk::slot::with(serde_json::json!({}), [slot]).into(),
    );
    state
}
#[test]
fn asked_panes_are_inline_at_both_widths_and_unasked_panes_wait_for_a_click() {
    for width in [80, 200] {
        let (mut ui, now) = scene();
        let tree = solo(&with(pane(true)));
        let screen = draw_tree(width, 24, &tree, &mut ui, now);
        assert_eq!(ui.shown.seating.panes[0].area.height, 10);
        assert_eq!(
            ui.shown.seating.panes[0].area.bottom(),
            ui.shown.regions.activity.y
        );
        assert!(
            screen.contains("Counter pane") && screen.contains("Count: 0"),
            "{screen}"
        );
    }
    let (mut ui, now) = scene();
    let tree = solo(&with(pane(false)));
    let screen = draw_tree(200, 24, &tree, &mut ui, now);
    assert!(screen.contains("1 pane waiting"), "{screen}");
    assert!(!screen.contains("Count: 0"));
    let effects = crate::pointer::on_mouse(
        &mut ui,
        &tree,
        crossterm::event::MouseEvent {
            kind: crossterm::event::MouseEventKind::Down(crossterm::event::MouseButton::Left),
            column: 198,
            row: 23,
            modifiers: crossterm::event::KeyModifiers::NONE,
        },
        now,
    );
    assert!(
        matches!(&effects[..], [crate::effect::Effect::Submit(bingo_sdk::Input::Action { action, .. })] if action.name == "counter:ui" && action.args["event"] == "open")
    );
    assert!(draw_tree(200, 24, &tree, &mut ui, now).contains("1 pane waiting"));
    let tree = solo(&with(pane(true)));
    assert!(draw_tree(200, 24, &tree, &mut ui, now).contains("Count: 0"));
}
#[test]
fn pane_keys_use_handlers_and_an_independent_composer() {
    use bingo_sdk::{
        Action,
        view::{ButtonProps, InputProps},
    };
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
    let action = |event: &str| Action {
        name: "counter:ui".into(),
        args: serde_json::json!({"handler":"token","event":event}),
    };
    let mut slot = pane(true);
    if let Slot::Pane { view, .. } = &mut slot {
        *view = View::column(vec![
            View::Button {
                children: vec![],
                button: ButtonProps {
                    hotkey: Some('i'),
                    on_press: Some(action("press")),
                    ..Default::default()
                }
                .into(),
            },
            View::Input {
                input: InputProps {
                    value: Some("A".into()),
                    on_input: Some(action("input")),
                    ..Default::default()
                }
                .into(),
            },
        ])
        .into();
    }
    let state = with(slot);
    let tree = solo(&state);
    let (mut ui, now) = scene();
    ui.sync_panes(&state);
    draw_tree(80, 24, &tree, &mut ui, now);
    let key = |code| KeyEvent::new(code, KeyModifiers::NONE);
    let fired = crate::input::on_key(&mut ui, &tree, key(KeyCode::Char('i')), now);
    assert!(
        matches!(&fired[..],[crate::effect::Effect::Submit(bingo_sdk::Input::Action{action, ..})] if action.args["event"]=="press")
    );
    crate::input::on_key(&mut ui, &tree, key(KeyCode::Tab), now);
    let typed = crate::input::on_key(&mut ui, &tree, key(KeyCode::Char('b')), now);
    assert!(
        matches!(&typed[..],[crate::effect::Effect::Submit(bingo_sdk::Input::Action{action, ..})] if action.args["value"]=="Ab")
    );
    assert!(ui.composer.is_empty());
    assert!(draw_tree(80, 24, &tree, &mut ui, now).contains("Ab"));
    let closed = crate::input::on_key(&mut ui, &tree, key(KeyCode::Esc), now);
    assert!(
        matches!(&closed[..],[crate::effect::Effect::Submit(bingo_sdk::Input::Action{action, ..})] if action.args["event"]=="close")
    );
}

/// One control alone in the counter pane, focused, and the effects of `key`.
fn pressed_on(control: View, key: crossterm::event::KeyCode) -> Vec<crate::effect::Effect> {
    let mut slot = pane(true);
    if let Slot::Pane { view, .. } = &mut slot {
        *view = View::column(vec![control]).into();
    }
    let state = with(slot);
    let tree = solo(&state);
    let (mut ui, now) = scene();
    ui.sync_panes(&state);
    draw_tree(80, 24, &tree, &mut ui, now);
    let key = crossterm::event::KeyEvent::new(key, crossterm::event::KeyModifiers::NONE);
    crate::input::on_key(&mut ui, &tree, key, now)
}

fn button(action: bingo_sdk::Action) -> View {
    View::Button {
        children: vec![],
        button: bingo_sdk::view::ButtonProps {
            on_press: Some(action),
            ..Default::default()
        }
        .into(),
    }
}

/// A plugin's own pane may draw a button naming any command; one Enter on it
/// is the plugin's word, so a person-only verb or a wider mode is refused by
/// the kernel rather than granted as the person's.
#[test]
fn a_pane_button_naming_another_command_is_signed_as_its_drawer() {
    use bingo_sdk::{Action, Input, Origin};
    use serde_json::json;
    let drawer = Some(Origin::surface("plugin:counter"));
    let person = Some(Origin::surface(crate::SURFACE_ID));
    for (name, args, signed) in [
        ("plugins", json!("trust evil full"), &drawer),
        ("permission", json!("bypassPermissions"), &drawer),
        (
            "plugins:ui",
            json!({"handler":"h","event":"press"}),
            &drawer,
        ),
        (
            "counter:ui",
            json!({"handler":"h","event":"press"}),
            &person,
        ),
    ] {
        let action = Action {
            name: name.into(),
            args,
        };
        let effects = pressed_on(button(action.clone()), crossterm::event::KeyCode::Enter);
        assert_eq!(
            effects,
            vec![crate::effect::Effect::Submit(Input::Action {
                action,
                origin: signed.clone(),
            })],
            "{name}"
        );
    }
}

/// A control whose arguments are not an object cannot carry the value typed
/// or chosen: nothing is sent, and nothing panics.
#[test]
fn a_select_or_input_over_arguments_that_are_not_an_object_sends_nothing() {
    use bingo_sdk::view::{InputProps, SelectOption, SelectProps};
    use crossterm::event::KeyCode;
    use serde_json::json;
    for args in [json!("text"), json!(42), json!([1]), json!(true)] {
        let action = bingo_sdk::Action {
            name: "counter:ui".into(),
            args,
        };
        let select = View::Select {
            select: SelectProps {
                options: Some(vec![SelectOption {
                    label: "One".into(),
                    value: "one".into(),
                }]),
                on_select: Some(action.clone()),
                ..Default::default()
            }
            .into(),
        };
        assert!(pressed_on(select, KeyCode::Enter).is_empty());
        let typed = View::Input {
            input: InputProps {
                on_input: Some(action.clone()),
                ..Default::default()
            }
            .into(),
        };
        assert!(pressed_on(typed, KeyCode::Char('x')).is_empty());
        let submitted = View::Input {
            input: InputProps {
                value: Some("x".into()),
                on_submit: Some(action),
                ..Default::default()
            }
            .into(),
        };
        assert!(pressed_on(submitted, KeyCode::Enter).is_empty());
    }
}
