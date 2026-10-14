use super::*;
use crate::{doubles::TestSession, test_support::*};
use bingo_sdk::{Action, component::PaletteOpen};
use crossterm::event::KeyCode;
use serde_json::json;
use std::sync::Arc;

fn pull(run: &mut Run, now: crate::clock::Now) {
    draw_tree(80, 24, &run.session.tree, &mut run.ui, now);
    let key = run
        .ui
        .shown
        .pulls
        .iter()
        .find(|key| key.request.component == bingo_sdk::render::Component::Palette)
        .unwrap()
        .clone();
    let view = View::Box {
        children: vec![View::text("palette")],
        layout: Box::new(bingo_sdk::view::BoxStyle {
            on_key: Some(Action {
                name: "palette:ui".into(),
                args: json!({"query":key.request.props["query"]}),
            }),
            ..Default::default()
        }),
    };
    run.ui.placed.pulled.request(&key);
    run.reply(Reply::Rendered(key, Box::new(view)), &mut None);
}

#[tokio::test]
async fn rapid_text_then_enter_waits_for_the_matching_palette_frame() {
    let now = still(scene().1);
    let mut state = state();
    state.config.kernel["render"] = json!({"Palette":[{}]});
    let session = Arc::new(TestSession::default());
    let mut run = tests::idle_in(state, session.clone(), now.instant);
    crate::palette::open(&mut run.ui, PaletteOpen::Commands, now);
    for key in [typed('m'), typed('o'), key(KeyCode::Enter)] {
        run.terminal_event(Term::Key(key));
    }
    assert!(session.submitted().is_empty());
    for (index, query) in ["/", "/m", "/mo"].into_iter().enumerate() {
        pull(&mut run, now);
        let submissions = session.submissions();
        assert_eq!(submissions.len(), index + 1);
        let (intent, Input::Action { action }) = &submissions[index] else {
            panic!("key action")
        };
        assert_eq!(action.args["query"], query);
        if index < 2 {
            run.ack(
                intent,
                &IntentOutcome::Applied {
                    result: json!({"handled":false}),
                },
            );
            assert_eq!(
                session.submitted().len(),
                index + 1,
                "wait for the new query's drawing"
            );
        } else {
            assert_eq!(action.args["value"]["key"], "enter");
        }
    }
    assert_eq!(run.ui.composer.text(), "/mo");
}

#[test]
fn a_session_list_reply_belongs_only_to_its_opening() {
    let now = still(scene().1);
    let mut run = tests::idle(now.instant);
    crate::palette::open(&mut run.ui, PaletteOpen::Sessions, now);
    let old = crate::palette::request(&run.ui).unwrap().to_owned();
    crate::palette::close(&mut run.ui, now);
    crate::palette::open(&mut run.ui, PaletteOpen::Sessions, now);
    palette::fill_palette(&mut run, &old, vec![summary()]);
    let Open::Palette(open) = run.ui.focus.open() else {
        panic!("palette")
    };
    assert!(open.sessions.is_empty());
    let current = open.id.clone();
    palette::fill_palette(&mut run, &current, vec![summary()]);
    let Open::Palette(open) = run.ui.focus.open() else {
        panic!("palette")
    };
    assert_eq!(open.sessions.len(), 1);
}

#[tokio::test]
async fn the_top_palette_owns_keys_when_an_ask_arrives_after_idle() {
    let now = still(scene().1);
    for first in [PaletteOpen::Commands, PaletteOpen::Keys] {
        for ask in [question(false, true), permission(None, None)] {
            for pressed in [typed('x'), key(KeyCode::Enter), key(KeyCode::Esc)] {
                let mut state = state();
                state.config.kernel["render"] = json!({"Palette":[{}]});
                let session = Arc::new(TestSession::default());
                let mut run = tests::idle_in(state, session.clone(), now.instant);
                crate::palette::open(&mut run.ui, first, now);
                run.ui.last_typed = Some(now.wall - std::time::Duration::from_secs(2));
                run.session
                    .tree
                    .fold
                    .apply(&child_frame(1, announced("asking child")));
                run.session
                    .tree
                    .fold
                    .apply(&child_frame(2, opened(ask.clone())));
                interactions::refocus_at(&mut run, now.wall);
                assert!(
                    run.ui.dialog.aside,
                    "the palette keeps the arriving ask aside"
                );
                pull(&mut run, now);
                let id = crate::palette::request(&run.ui).unwrap().to_owned();
                run.terminal_event(Term::Key(pressed));
                let submissions = session.submissions();
                let (intent, Input::Action { .. }) = &submissions[0] else {
                    panic!("palette action")
                };
                let result = if pressed.code == KeyCode::Char('x') {
                    json!({"handled":false})
                } else {
                    json!({"surface":{"name":"surface:palette","args":{"handler":id,"value":{"close":true}}}})
                };
                run.ack(intent, &IntentOutcome::Applied { result });
                assert!(run.session.tree.open_interaction().is_some());
                assert_eq!(
                    session.submissions().len(),
                    1,
                    "no answer or interrupt escaped"
                );
                if pressed.code == KeyCode::Char('x') {
                    assert!(run.ui.composer.text().ends_with('x'));
                    assert!(run.ui.dialog.aside);
                } else {
                    assert!(!crate::palette::active(&run.ui));
                    assert!(!run.ui.dialog.aside, "closing returns focus to the ask");
                }
            }
        }
    }
}

#[tokio::test]
async fn an_unmatched_palette_replacement_never_queues_the_keyboard() {
    let now = still(scene().1);
    let mut state = state();
    state.config.kernel["render"] = json!({"Palette":[{"props":{"open":"sessions"}}]});
    let mut run = tests::idle_in(state, Arc::new(TestSession::default()), now.instant);
    crate::palette::open(&mut run.ui, PaletteOpen::Commands, now);
    draw_tree(80, 24, &run.session.tree, &mut run.ui, now);
    run.terminal_event(Term::Key(typed('m')));
    assert_eq!(run.ui.composer.text(), "/m");
    run.terminal_event(Term::Key(key(KeyCode::Esc)));
    assert!(!crate::palette::active(&run.ui));
    assert!(run.site_queue.is_empty());
}
