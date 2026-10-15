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

#[tokio::test]
async fn publishing_an_open_pane_requests_focus_again_without_duplicating_it() {
    let now = scene().1;
    let mut run = tests::idle(now.instant);
    let mut screen = Recorder::default();
    for seq in 1..=3 {
        if seq < 3 {
            run.ui.focus = Focus::Composer;
        }
        run.frame(&frame(seq, publication(true)), &mut screen)
            .unwrap();
        run.ui.sync_panes(run.session.tree.viewed());
        let drawn = draw_tree(108, 30, &run.session.tree, &mut run.ui, now);
        assert!(drawn.contains("Key-only pane"));
        assert!(matches!(run.ui.focus, Focus::Pane(_)), "publication {seq}");
        assert_eq!(crate::pane::placed(run.session.tree.viewed()).len(), 1);
    }
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
