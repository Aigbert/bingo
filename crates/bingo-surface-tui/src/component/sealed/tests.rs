use super::*;
use crate::{effect::Effect, test_support::*};
use bingo_sdk::{Answer, AnswerSpec, ItemStatus, LoginFlow, render::Component};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers, MouseButton, MouseEvent, MouseEventKind};
use serde_json::json;

fn state_with_card(login: bool) -> SessionState {
    let open = if login {
        interaction(
            InteractionKind::Login {
                provider: "example".into(),
                flow: LoginFlow::Device {
                    url: "https://example.test/device".into(),
                    code: "ABCD".into(),
                },
            },
            vec![AnswerSpec::Cancel],
        )
    } else {
        interaction(
            InteractionKind::Permission {
                tool: "sample".into(),
                summary: "sample operation".into(),
                preview: None,
                session_scope: Some("sample".into()),
                caution: None,
                input: json!({"secret":"full input"}),
            },
            vec![
                AnswerSpec::AllowOnce,
                AnswerSpec::AllowSession,
                AnswerSpec::Deny,
            ],
        )
    };
    let mut state = folded(vec![
        started_tool(
            1,
            tool(
                "itm_2",
                "sample",
                json!({"secret":"full input"}),
                None,
                ItemStatus::Running,
            ),
        ),
        frame(2, opened(open)),
    ]);
    state.config.kernel["render"] = json!({"Permission":[{}],"SignIn":[{}]});
    state
}

fn install(ui: &mut crate::ui::Ui, key: &crate::pulled::Key, answer: View) {
    ui.placed.pulled.request(key);
    ui.placed.pulled.answer(key.clone(), answer);
}

#[test]
fn framed_cards_keep_host_facts_and_keys_at_both_sizes() {
    for login in [false, true] {
        for (width, height) in [(80, 24), (200, 50)] {
            let state = state_with_card(login);
            let tree = solo(&state);
            let (mut ui, now) = scene();
            let now = still(now);
            ui.dialog.focus_on(state.interactions.first());
            draw_tree(width, height, &tree, &mut ui, now);
            let key = ui
                .shown
                .pulls
                .iter()
                .find(|key| key.request.component.sealed())
                .unwrap()
                .clone();
            let answer = View::column(vec![View::text("SEALED FRAME"), key.request.engine()]);
            install(&mut ui, &key, answer);
            let screen = draw_tree(width, height, &tree, &mut ui, now);
            assert!(screen.contains("SEALED FRAME"), "{screen}");
            assert!(
                screen.contains(if login { "ABCD" } else { "full input" }),
                "{screen}"
            );
            let code = if login {
                KeyCode::Esc
            } else {
                KeyCode::Char('y')
            };
            let effects =
                crate::input::on_key(&mut ui, &tree, KeyEvent::new(code, KeyModifiers::NONE), now);
            assert!(
                matches!(&effects[0], Effect::Answer { answer, .. } | Effect::StopInteraction { answer, .. } if *answer == if login { Answer::Cancel } else { Answer::AllowOnce })
            );
        }
    }
}

#[test]
fn clipped_rewritten_missing_and_duplicate_engines_fall_back_once() {
    for login in [false, true] {
        let state = state_with_card(login);
        let tree = solo(&state);
        let (mut ui, now) = scene();
        let now = still(now);
        draw_tree(80, 24, &tree, &mut ui, now);
        let key = ui
            .shown
            .pulls
            .iter()
            .find(|key| key.request.component.sealed())
            .unwrap()
            .clone();
        let engine = serde_json::to_value(key.request.engine()).unwrap();
        let cases = [
            json!({"kind":"Box","height":1,"children":[engine.clone()]}),
            json!({"kind":"Box","width":0,"children":[engine.clone()]}),
            json!({"kind":"Box","display":"none","children":[engine.clone()]}),
            json!({"kind":"Box","children":[engine.clone(),engine]}),
            json!({"kind":"Text","children":["FAKE CARD"]}),
            json!({"kind":"Engine","component":key.request.component,"props":{}}),
        ];
        for answer in cases {
            ui.dialog = dialog::Dialog::default();
            ui.dialog.focus_on(state.interactions.first());
            install(&mut ui, &key, serde_json::from_value(answer).unwrap());
            let screen = draw_tree(80, 24, &tree, &mut ui, now);
            assert_eq!(screen.matches("showing host card").count(), 1, "{screen}");
            assert!(!screen.contains("FAKE CARD"));
            assert!(ui.shown.card_actions.is_empty());
            let code = if login {
                KeyCode::Esc
            } else {
                KeyCode::Char('y')
            };
            assert!(
                crate::input::on_key(&mut ui, &tree, KeyEvent::new(code, KeyModifiers::NONE), now)
                    .iter()
                    .any(|effect| matches!(
                        effect,
                        Effect::Answer { .. } | Effect::StopInteraction { .. }
                    ))
            );
        }
    }
}

#[test]
fn a_button_gets_its_own_click_but_cannot_take_the_yes_key() {
    let state = state_with_card(false);
    let tree = solo(&state);
    let (mut ui, now) = scene();
    let now = still(now);
    ui.dialog.focus_on(state.interactions.first());
    draw_tree(80, 24, &tree, &mut ui, now);
    let key = ui
        .shown
        .pulls
        .iter()
        .find(|key| key.request.component == Component::Permission)
        .unwrap()
        .clone();
    let button: View = serde_json::from_value(json!({"kind":"Button","children":["Yes"],"hotkey":"y","onPress":{"name":"frame:ui","args":{"handler":"answer","event":"press"}}})).unwrap();
    install(
        &mut ui,
        &key,
        View::column(vec![button, key.request.engine()]),
    );
    draw_tree(80, 24, &tree, &mut ui, now);
    let at = ui.shown.card_actions.first().unwrap().area;
    let effects = crate::pointer::on_mouse(
        &mut ui,
        &tree,
        MouseEvent {
            kind: MouseEventKind::Down(MouseButton::Left),
            column: at.x,
            row: at.y,
            modifiers: KeyModifiers::NONE,
        },
        now,
    );
    assert!(
        matches!(&effects[..], [Effect::Submit(bingo_sdk::Input::Action { action })] if action.name == "frame:ui")
    );
    assert!(!ui.dialog.answered);
    let effects = crate::input::on_key(
        &mut ui,
        &tree,
        KeyEvent::new(KeyCode::Char('y'), KeyModifiers::NONE),
        now,
    );
    assert!(matches!(
        &effects[..],
        [Effect::Answer {
            answer: Answer::AllowOnce,
            ..
        }]
    ));
}

#[test]
fn unmatched_cards_fall_back_and_input_needs_no_transcript_item() {
    let mut state = state_with_card(false);
    state.config.kernel["render"] = json!({});
    let tree = solo(&state);
    let (mut ui, now) = scene();
    let before = draw_tree(80, 24, &tree, &mut ui, still(now));
    assert!(ui.shown.pulls.is_empty());
    state.config.kernel["render"] = json!({"Permission":[{}]});
    let tree = solo(&state);
    let after = draw_tree(80, 24, &tree, &mut ui, still(now));
    assert_eq!(before, after);
    let mut open = state.interactions[0].clone();
    open.item = None;
    assert_eq!(props(&state, &open), props(&state, &state.interactions[0]));
}

#[test]
fn resizing_rechecks_height_and_stale_props_never_draw() {
    let state = state_with_card(false);
    let tree = solo(&state);
    let (mut ui, now) = scene();
    let now = still(now);
    draw_tree(80, 24, &tree, &mut ui, now);
    let key = ui
        .shown
        .pulls
        .iter()
        .find(|key| key.request.component.sealed())
        .unwrap()
        .clone();
    install(
        &mut ui,
        &key,
        View::column(vec![View::text("SEALED FRAME"), key.request.engine()]),
    );
    assert!(draw_tree(80, 24, &tree, &mut ui, now).contains("SEALED FRAME"));
    let short = draw_tree(80, 10, &tree, &mut ui, now);
    assert!(!short.contains("SEALED FRAME"));
    assert!(ui.shown.card_actions.is_empty());
    assert!(draw_tree(80, 24, &tree, &mut ui, now).contains("SEALED FRAME"));
    let mut changed = state.clone();
    changed.interactions[0].answers = vec![AnswerSpec::Deny];
    assert!(!draw_tree(80, 24, &solo(&changed), &mut ui, now).contains("SEALED FRAME"));
}

#[test]
fn host_option_clicks_use_engine_cells() {
    let state = state_with_card(false);
    let tree = solo(&state);
    let (mut ui, now) = scene();
    let now = still(now);
    ui.dialog.focus_on(state.interactions.first());
    draw_tree(80, 24, &tree, &mut ui, now);
    let key = ui
        .shown
        .pulls
        .iter()
        .find(|key| key.request.component.sealed())
        .unwrap()
        .clone();
    install(
        &mut ui,
        &key,
        View::column(vec![
            View::text("host choices follow"),
            key.request.engine(),
        ]),
    );
    draw_tree(80, 24, &tree, &mut ui, now);
    let (at, _) = ui
        .shown
        .card
        .as_ref()
        .unwrap()
        .sealed_options
        .iter()
        .find(|(_, option)| *option == 0)
        .unwrap();
    let mouse = MouseEvent {
        kind: MouseEventKind::Down(MouseButton::Left),
        column: at.x,
        row: at.y,
        modifiers: KeyModifiers::NONE,
    };
    let effects = crate::pointer::on_mouse(&mut ui, &tree, mouse, now);
    assert!(matches!(
        &effects[..],
        [Effect::Answer {
            answer: Answer::AllowOnce,
            ..
        }]
    ));
}

#[test]
fn paste_keeps_the_host_editor() {
    let mut state = state_with_card(true);
    let (mut ui, now) = scene();
    let now = still(now);
    state.interactions[0].kind = InteractionKind::Login {
        provider: "example".into(),
        flow: LoginFlow::Paste,
    };
    state.interactions[0].answers = vec![AnswerSpec::Text, AnswerSpec::Cancel];
    ui.dialog = dialog::Dialog::default();
    ui.dialog.focus_on(state.interactions.first());
    let tree = solo(&state);
    draw_tree(80, 24, &tree, &mut ui, now);
    let key = ui
        .shown
        .pulls
        .iter()
        .find(|key| key.request.component == Component::SignIn)
        .unwrap()
        .clone();
    install(
        &mut ui,
        &key,
        View::column(vec![View::text("sign-in frame"), key.request.engine()]),
    );
    assert!(draw_tree(80, 24, &tree, &mut ui, now).contains("sign-in frame"));
    crate::input::on_key(
        &mut ui,
        &tree,
        KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE),
        now,
    );
    assert!(ui.dialog.words.is_some());
    for ch in "test-token".chars() {
        crate::input::on_key(
            &mut ui,
            &tree,
            KeyEvent::new(KeyCode::Char(ch), KeyModifiers::NONE),
            now,
        );
    }
    let effects = crate::input::on_key(
        &mut ui,
        &tree,
        KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE),
        now,
    );
    assert!(
        matches!(&effects[..],[Effect::Answer {answer:Answer::Text {text},..}] if text == "test-token")
    );
}
