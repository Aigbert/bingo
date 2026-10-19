use super::*;
use crate::test_support::*;

#[test]
fn up_keeps_a_draft_even_at_the_first_line() {
    let (mut ui, now) = scene();
    ui.history.remember("older");
    ui.composer.set("draft");
    let tree = solo(&state());
    assert!(on_key(&mut ui, &tree, key(KeyCode::Up), now).is_empty());
    assert_eq!(ui.composer.text(), "draft");
}

#[test]
fn clearing_a_draft_puts_it_in_history() {
    let (mut ui, now) = scene();
    ui.composer.set("keep this");
    let tree = solo(&state());
    on_key(&mut ui, &tree, key(KeyCode::Esc), now);
    assert!(ui.composer.is_empty());
    assert_eq!(
        ui.notice().unwrap().plain().unwrap().text,
        "cleared · ↑ brings it back"
    );
    on_key(&mut ui, &tree, key(KeyCode::Up), now);
    assert_eq!(ui.composer.text(), "keep this");
    on_key(&mut ui, &tree, key(KeyCode::Down), now);
    assert!(ui.composer.is_empty());
}

#[test]
fn ctrl_c_uses_the_frame_clock_and_stopping_disarms_exit() {
    let (mut ui, now) = scene();
    let tree = solo(&state());
    let ctrl_c = KeyEvent::new(KeyCode::Char('c'), KeyModifiers::CONTROL);
    on_key(&mut ui, &tree, ctrl_c, now);
    assert_eq!(
        ui.notice().unwrap().plain().unwrap().text,
        "ctrl+c again to quit"
    );
    assert_eq!(
        on_key(&mut ui, &tree, ctrl_c, later(now, 1499)),
        vec![Effect::Exit]
    );
    assert!(on_key(&mut ui, &tree, ctrl_c, later(now, 1500)).is_empty());
    let busy = solo(&folded(vec![frame(1, started("trn_1"))]));
    assert_eq!(
        on_key(&mut ui, &busy, ctrl_c, later(now, 1510)),
        vec![Effect::Stop { withdraw: vec![] }]
    );
    assert!(ui.armed.is_none());
    assert!(on_key(&mut ui, &tree, ctrl_c, later(now, 1520)).is_empty());
}

#[test]
fn question_mark_types_while_a_turn_runs() {
    let (mut ui, now) = scene();
    let tree = solo(&folded(vec![frame(1, started("trn_1"))]));
    assert!(on_key(&mut ui, &tree, typed('?'), now).is_empty());
    assert_eq!(ui.composer.text(), "?");
}

#[test]
fn alt_enter_keeps_the_composers_newline_binding() {
    let (mut ui, now) = scene();
    ui.composer.set("draft");
    let tree = solo(&state());
    on_key(
        &mut ui,
        &tree,
        KeyEvent::new(KeyCode::Enter, KeyModifiers::ALT),
        now,
    );
    assert_eq!(ui.composer.text(), "draft\n");
}

#[test]
fn legacy_esc_enter_returns_to_an_aside_ask_then_answers() {
    let (mut ui, now) = scene();
    let tree = solo(&folded(vec![frame(
        1,
        bingo_sdk::Event::InteractionOpened {
            interaction: permission(None, None),
        },
    )]));
    ui.dialog.aside = true;
    let effects = on_key(
        &mut ui,
        &tree,
        KeyEvent::new(KeyCode::Enter, KeyModifiers::ALT),
        later(now, 1000),
    );
    assert!(!ui.dialog.aside);
    assert!(
        matches!(effects.as_slice(), [Effect::Answer { .. }]),
        "{effects:?}"
    );
}

#[test]
fn stop_withdraws_the_home_queue_in_order_and_preserves_a_draft() {
    let mut state = folded(vec![frame(1, started("trn_1"))]);
    let entries = (0..3)
        .map(|n| bingo_sdk::QueueEntry {
            intent: IntentId::from_raw(format!("q{n}")),
            position: n + 1,
            preview: format!("line {n}"),
            steerable: n == 0,
            origin: crate::origin(),
        })
        .collect();
    state.apply(&frame(
        2,
        bingo_sdk::Event::QueueChanged {
            revision: 1,
            entries,
        },
    ));
    let tree = solo(&state);
    for stop in [
        key(KeyCode::Esc),
        KeyEvent::new(KeyCode::Char('c'), KeyModifiers::CONTROL),
    ] {
        let (mut ui, now) = scene();
        assert_eq!(
            on_key(&mut ui, &tree, stop, now),
            vec![Effect::Stop {
                withdraw: vec![
                    IntentId::from_raw("q0"),
                    IntentId::from_raw("q1"),
                    IntentId::from_raw("q2"),
                ]
            }]
        );
        ui.composer.set("draft");
        assert_eq!(
            on_key(&mut ui, &tree, stop, now),
            vec![Effect::Stop { withdraw: vec![] }]
        );
        assert_eq!(ui.composer.text(), "draft");
        assert!(ui.armed.is_none() && ui.escaped_at.is_none());
    }
}

#[test]
fn escape_times_rewind_and_scroll_takes_precedence() {
    let (state, commands) = super::tests::rewindable();
    let tree = solo(&state);
    for (delay, opens) in [(599, true), (600, false)] {
        let (mut ui, now) = scene();
        ui.catalogs.commands = crate::commands::entries(commands.clone());
        on_key(&mut ui, &tree, key(KeyCode::Esc), now);
        assert_eq!(
            ui.notice().unwrap().plain().unwrap().text,
            "esc again to rewind"
        );
        on_key(&mut ui, &tree, key(KeyCode::Esc), later(now, delay));
        assert_eq!(ui.focus.showing(), opens);
    }
    let (mut ui, now) = scene();
    ui.scroll = crate::scroll::Scroll::Held {
        from: 0,
        to: 0,
        since: now.instant,
    };
    on_key(&mut ui, &tree, key(KeyCode::Esc), now);
    assert!(ui.scroll.following());
    assert!(ui.escaped_at.is_none());
}

#[test]
fn ctrl_c_closes_then_clears_and_an_unbound_alt_enter_splits() {
    let tree = solo(&state());
    let (mut ui, now) = scene();
    ui.composer.set("draft");
    ui.focus = crate::focus::Focus::Search(Search::open());
    let ctrl_c = KeyEvent::new(KeyCode::Char('c'), KeyModifiers::CONTROL);
    on_key(&mut ui, &tree, ctrl_c, now);
    assert!(matches!(ui.focus, crate::focus::Focus::Composer));
    assert_eq!(ui.composer.text(), "draft");
    on_key(&mut ui, &tree, ctrl_c, now);
    assert!(ui.composer.is_empty());
    ui.focus = crate::focus::Focus::Search(Search::open());
    ui.composer.set("send me");
    let effects = on_key(
        &mut ui,
        &tree,
        KeyEvent::new(KeyCode::Enter, KeyModifiers::ALT),
        now,
    );
    assert!(
        matches!(effects.as_slice(), [Effect::Submit(_)]),
        "{effects:?}"
    );
}

#[test]
fn a_model_without_images_refuses_paste_and_names_the_switch_key() {
    let mut state = state();
    state.summary.model = Some("text-only".into());
    let (mut ui, now) = scene();
    ui.catalogs.values.insert(
        "models".into(),
        vec![bingo_sdk::CatalogEntry {
            id: "fake/text-only".into(),
            label: "text-only".into(),
            meta: serde_json::json!({"images":false}),
        }],
    );
    let effects = on_key(
        &mut ui,
        &solo(&state),
        KeyEvent::new(KeyCode::Char('v'), KeyModifiers::CONTROL),
        now,
    );
    assert_eq!(effects, vec![Effect::PasteImage]);
    assert!(ui.notice().is_none());
}

#[test]
fn ctrl_c_and_escape_send_the_same_interaction_cancel() {
    let tree = solo(&folded(vec![frame(1, opened(permission(None, None)))]));
    let (mut escape_ui, now) = scene();
    let (mut interrupt_ui, _) = scene();
    let escape = on_key(&mut escape_ui, &tree, key(KeyCode::Esc), now);
    let interrupt = on_key(
        &mut interrupt_ui,
        &tree,
        KeyEvent::new(KeyCode::Char('c'), KeyModifiers::CONTROL),
        now,
    );
    assert!(!escape.is_empty());
    assert_eq!(interrupt, escape);
}
