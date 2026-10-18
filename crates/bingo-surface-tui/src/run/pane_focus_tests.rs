use super::*;
use crate::{
    doubles::{Recorder, TestSession},
    focus::{Focus, Owner},
    test_support::*,
};
use serde_json::json;
use std::sync::Arc;

fn publication(asked: bool) -> Event {
    Event::Signal {
        generation: None,
        plugin: "fixture".into(),
        kind: "pane".into(),
        payload: json!({"slots":[{
            "kind":"pane","id":"keys","title":"Keys","focus":true,"asked":asked,
            "closeOnEscape":true,"holdToasts":false,"rows":4,
            "view":{"kind":"Box","onKey":{"name":"fixture:ui","args":{}},
                "children":[{"kind":"Text","children":["Key-only pane"]}]}
        }]}),
    }
}

fn hub_publication() -> Event {
    let rows = bingo_mod_status::rows::rows(&(&state()).into(), &[], &[], None);
    let view = bingo_mod_status::draw::tree(&rows, "session", 108, 14, 0);
    let Event::Signal { mut payload, .. } = publication(true) else {
        unreachable!()
    };
    payload["slots"][0]["id"] = json!("status");
    payload["slots"][0]["rows"] = json!(14);
    payload["slots"][0]["view"] = json!(view);
    Event::Signal {
        generation: None,
        plugin: bingo_sdk::Plugin::manifest(&bingo_mod_status::Status::default())
            .id
            .into(),
        kind: "status".into(),
        payload,
    }
}

#[tokio::test]
async fn refresh_preserves_a_draft_and_only_own_opener_receipt_refocuses() {
    let now = scene().1;
    let mut run = tests::idle(now.instant);
    let mut screen = Recorder::default();
    run.frame(&frame(1, hub_publication()), &mut screen)
        .unwrap();
    run.ui.sync_panes(run.session.tree.viewed());
    draw_tree(108, 30, &run.session.tree, &mut run.ui, now);
    run.ui.focus = Focus::Composer;
    run.ui.composer.set("a draft");
    run.frame(&frame(2, hub_publication()), &mut screen)
        .unwrap();
    run.ui.sync_panes(run.session.tree.viewed());
    let drawn = draw_tree(108, 30, &run.session.tree, &mut run.ui, now);
    assert!(drawn.contains("spent"));
    assert!(matches!(run.ui.focus, Focus::Composer));
    assert_eq!(run.ui.composer.text(), "a draft");
    let pane = crate::pane::placed(run.session.tree.viewed()).remove(0);
    let outcome = IntentOutcome::Applied {
        result: json!({"surface":{"name":"Pane","args":{"plugin":pane.plugin,"id":pane.id}}}),
    };
    run.ack(&IntentId::mint(), &outcome);
    assert!(matches!(run.ui.focus, Focus::Composer));
    let intent = run.mint(crate::run::Submission::Ordinary(Some("/status".into())));
    run.ack(&intent, &outcome);
    draw_tree(108, 30, &run.session.tree, &mut run.ui, now);
    assert!(matches!(run.ui.focus, Focus::Pane(_)));
    assert_eq!(crate::pane::placed(run.session.tree.viewed()).len(), 1);
    assert_eq!(run.ui.composer.text(), "a draft");
    run.ui.focus = Focus::Composer;
    run.ack(&intent, &outcome);
    assert!(matches!(run.ui.focus, Focus::Composer));
}

#[tokio::test]
async fn pane_publications_and_focus_requests_leave_asks_unanswered() {
    for open in [
        permission(None, None),
        question(false, true),
        login(bingo_sdk::LoginFlow::Paste),
    ] {
        for asked in [false, true] {
            let now = still(scene().1);
            let session = Arc::new(TestSession::default());
            let mut state = state();
            state.apply(&frame(1, opened(open.clone())));
            let mut run = tests::idle_in(state, session.clone(), now.instant);
            let mut screen = Recorder::default();
            run.frame(&frame(2, publication(asked)), &mut screen)
                .unwrap();
            run.ui.sync_panes(run.session.tree.viewed());
            run.ui.focus = Focus::Composer;
            surface::action(
                &mut run,
                bingo_sdk::Action {
                    name: "Pane".into(),
                    args: json!({"plugin":"fixture","id":"keys"}),
                },
            );
            assert!(matches!(run.ui.focus, Focus::Composer));
            draw_tree(108, 30, &run.session.tree, &mut run.ui, now);
            let area = run
                .ui
                .shown
                .seating
                .panes
                .first()
                .map(|seat| seat.area)
                .unwrap_or_else(|| run.ui.shown.waiting[0].1);
            let effects = crate::pointer::on_mouse(
                &mut run.ui,
                &run.session.tree,
                crossterm::event::MouseEvent {
                    kind: crossterm::event::MouseEventKind::Down(
                        crossterm::event::MouseButton::Left,
                    ),
                    column: area.x,
                    row: area.y,
                    modifiers: crossterm::event::KeyModifiers::NONE,
                },
                now,
            );
            assert!(effects.is_empty());
            for tab in [
                key(crossterm::event::KeyCode::Tab),
                shift(crossterm::event::KeyCode::BackTab),
            ] {
                let effects = input::on_key(&mut run.ui, &run.session.tree, tab, now);
                assert!(effects.is_empty());
                assert!(matches!(run.ui.focus, Focus::Composer));
            }
            assert_eq!(
                crate::focus::owner(&run.ui, &run.session.tree),
                Owner::Interaction
            );
            assert!(session.answers().is_empty());
            assert!(session.submitted().is_empty());
            assert_eq!(run.session.tree.viewed().interactions.len(), 1);
        }
    }
}

#[tokio::test]
async fn unasked_publications_and_redraws_do_not_request_focus() {
    let now = scene().1;
    let mut run = tests::idle(now.instant);
    let mut screen = Recorder::default();
    run.frame(&frame(1, publication(false)), &mut screen)
        .unwrap();
    run.ui.sync_panes(run.session.tree.viewed());
    draw_tree(108, 30, &run.session.tree, &mut run.ui, now);
    assert!(matches!(run.ui.focus, Focus::Composer));
    run.frame(&frame(2, publication(true)), &mut screen)
        .unwrap();
    run.ui.sync_panes(run.session.tree.viewed());
    run.ui.focus = Focus::Composer;
    run.frame(
        &frame(
            3,
            Event::Invalidated {
                requests: vec!["keys".into()],
            },
        ),
        &mut screen,
    )
    .unwrap();
    run.ui.sync_panes(run.session.tree.viewed());
    draw_tree(108, 30, &run.session.tree, &mut run.ui, now);
    assert!(matches!(run.ui.focus, Focus::Composer));
}

#[tokio::test]
async fn opener_receipt_consumes_the_pending_initial_focus() {
    let now = scene().1;
    let mut run = tests::idle(now.instant);
    let mut screen = Recorder::default();
    run.ui.focus = Focus::Layer(crate::ui::Layer::shut(now.instant));
    run.frame(&frame(1, publication(true)), &mut screen)
        .unwrap();
    run.ui.sync_panes(run.session.tree.viewed());
    let intent = run.mint(crate::run::Submission::Ordinary(Some("/open-keys".into())));
    run.ack(
        &intent,
        &IntentOutcome::Applied {
            result: json!({"surface":{"name":"Pane","args":{"plugin":"fixture","id":"keys"}}}),
        },
    );
    draw_tree(108, 30, &run.session.tree, &mut run.ui, now);
    assert!(matches!(run.ui.focus, Focus::Pane(_)));
    run.ui.focus = Focus::Composer;
    run.ui.composer.set("draft");
    run.frame(&frame(2, publication(true)), &mut screen)
        .unwrap();
    run.ui.sync_panes(run.session.tree.viewed());
    draw_tree(108, 30, &run.session.tree, &mut run.ui, now);
    assert!(matches!(run.ui.focus, Focus::Composer));
    assert_eq!(run.ui.composer.text(), "draft");
}

#[tokio::test]
async fn opener_receipt_addresses_held_keys_before_releasing_them() {
    for expired in [false, true] {
        let now = still(scene().1);
        let session = Arc::new(TestSession::default());
        let mut run = tests::idle_in(state(), session.clone(), now.instant);
        let mut screen = Recorder::default();
        run.frame(&frame(1, publication(true)), &mut screen)
            .unwrap();
        run.ui.sync_panes(run.session.tree.viewed());
        draw_tree(108, 30, &run.session.tree, &mut run.ui, now);
        run.ui.focus = Focus::Composer;
        let intent = run.mint(crate::run::Submission::Ordinary(Some("/open-keys".into())));
        typeahead::command_sent(&mut run, intent.clone());
        run.terminal_event(Term::Key(typed('x')));
        assert_eq!(run.typeahead.len(), 1);
        if expired {
            run.typeahead.expire();
        }
        run.ack(
            &intent,
            &IntentOutcome::Applied {
                result: json!({"surface":{"name":"Pane","args":{"plugin":"fixture","id":"keys"}}}),
            },
        );
        draw_tree(108, 30, &run.session.tree, &mut run.ui, now);
        assert!(matches!(run.ui.focus, Focus::Pane(_)));
        assert_eq!(run.ui.composer.text(), "");
        assert!(run.typeahead.is_empty());
        assert!(session.submitted().iter().any(|input| matches!(input,
            Input::Action { action, .. } if action.name == "fixture:ui" && action.args["value"]["key"] == "x"
        )));
    }
}

#[tokio::test]
async fn queued_opener_refocuses_only_when_its_final_receipt_arrives() {
    let now = scene().1;
    let mut run = tests::idle(now.instant);
    let mut screen = Recorder::default();
    run.frame(&frame(1, publication(true)), &mut screen)
        .unwrap();
    run.ui.sync_panes(run.session.tree.viewed());
    draw_tree(108, 30, &run.session.tree, &mut run.ui, now);
    run.ui.focus = Focus::Composer;
    run.ui.composer.set("draft");
    let intent = run.mint(crate::run::Submission::Ordinary(Some("/open-keys".into())));
    run.ack(&intent, &IntentOutcome::Queued { position: 1 });
    run.frame(&frame(2, publication(true)), &mut screen)
        .unwrap();
    draw_tree(108, 30, &run.session.tree, &mut run.ui, now);
    assert!(matches!(run.ui.focus, Focus::Composer));
    run.ack(
        &intent,
        &IntentOutcome::Applied {
            result: json!({"surface":{"name":"Pane","args":{"plugin":"fixture","id":"keys"}}}),
        },
    );
    draw_tree(108, 30, &run.session.tree, &mut run.ui, now);
    assert!(matches!(run.ui.focus, Focus::Pane(_)));
    assert_eq!(run.ui.composer.text(), "draft");
    assert!(!run.mine.contains_key(&intent));
}

#[tokio::test]
async fn queued_opener_releases_typeahead_but_keeps_its_final_receipt() {
    let now = still(scene().1);
    let mut run = tests::idle(now.instant);
    let intent = run.mint(crate::run::Submission::Ordinary(Some("/open-keys".into())));
    typeahead::command_sent(&mut run, intent.clone());
    run.terminal_event(Term::Key(typed('a')));
    assert_eq!(run.typeahead.len(), 1);
    run.ack(&intent, &IntentOutcome::Queued { position: 1 });
    draw_tree(108, 30, &run.session.tree, &mut run.ui, now);
    assert!(run.typeahead.deadline().is_none());
    assert!(run.typeahead.is_empty());
    assert_eq!(run.ui.composer.text(), "a");
    assert!(run.mine.contains_key(&intent));
    run.ack(&intent, &IntentOutcome::Applied { result: json!({}) });
    assert!(!run.mine.contains_key(&intent));
}
