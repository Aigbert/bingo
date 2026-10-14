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
