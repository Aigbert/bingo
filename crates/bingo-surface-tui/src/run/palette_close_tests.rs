use super::*;
use crate::{doubles::TestSession, palette::tests::Fixture, test_support::*};
use bingo_sdk::{CommandContext, CommandOutcome, component::PaletteOpen};
use std::sync::Arc;

async fn answer(run: &mut Run, session: &TestSession, fixture: &Fixture, index: usize) {
    let submissions = session.submissions();
    let (intent, Input::Action { action }) = &submissions[index] else {
        panic!("the run must send a palette action")
    };
    let outcome = fixture
        .handler
        .run(
            &action.args.to_string(),
            &CommandContext {
                session: run.session.tree.view().clone(),
                cwd: "/unused".into(),
                host: bingo_sdk::testing::NoHost::handle(),
                cancel: Default::default(),
            },
        )
        .await
        .unwrap();
    let result = match outcome {
        CommandOutcome::Surface { action, .. } => serde_json::json!({"surface":action}),
        CommandOutcome::Handled { handled } => serde_json::json!({"handled":handled}),
        _ => panic!("palette acknowledgement"),
    };
    run.ack(intent, &IntentOutcome::Applied { result });
}

async fn selected(run: &mut Run, fixture: &Fixture, now: crate::clock::Now) -> String {
    fixture
        .paint_tree(&mut run.ui, &run.session.tree, now, 80, 24)
        .await
        .lines()
        .find(|line| line.contains('❯'))
        .unwrap()
        .trim()
        .into()
}

#[tokio::test]
async fn ctrl_c_preserves_the_draft_and_retires_the_mod_cursor_in_the_run_loop() {
    for busy in [false, true] {
        let now = still(scene().1);
        let mut state = state();
        state.config.kernel["render"] = serde_json::json!({"Palette":[{}]});
        if busy {
            state.apply(&frame(1, started("trn_1")));
        }
        let session = Arc::new(TestSession::default());
        let mut run = tests::idle_in(state, session.clone(), now.instant);
        let fixture = Fixture::new();
        run.ui.composer.set("keep this draft");
        crate::palette::open(&mut run.ui, PaletteOpen::Keys, now);
        let opening = run.ui.focus.open().clone();
        let first = selected(&mut run, &fixture, now).await;
        run.terminal_event(Term::Key(key(crossterm::event::KeyCode::Down)));
        answer(&mut run, &session, &fixture, 0).await;
        assert_ne!(selected(&mut run, &fixture, now).await, first);
        run.terminal_event(Term::Key(ctrl('c')));
        assert!(!crate::palette::active(&run.ui));
        assert_eq!(run.ui.composer.text(), "keep this draft");
        assert_eq!(session.interrupts(), usize::from(busy));
        assert_eq!(session.submissions().len(), 2, "the owner receives close");
        answer(&mut run, &session, &fixture, 1).await;
        // Reuse the request only to observe that the mod forgot its old cursor.
        run.ui.focus.show(opening, now.instant);
        run.ui.composer.set("");
        assert_eq!(selected(&mut run, &fixture, now).await, first);
    }
}

#[tokio::test]
async fn a_palette_render_finishing_after_ctrl_c_is_retired() {
    let now = still(scene().1);
    let mut state = state();
    state.config.kernel["render"] = serde_json::json!({"Palette":[{}]});
    let session = Arc::new(TestSession::default());
    let mut run = tests::idle_in(state, session.clone(), now.instant);
    let fixture = Fixture::new();
    crate::palette::open(&mut run.ui, PaletteOpen::Keys, now);
    selected(&mut run, &fixture, now).await;
    let key = run
        .ui
        .shown
        .pulls
        .iter()
        .find(|key| key.request.component == bingo_sdk::render::Component::Palette)
        .unwrap()
        .clone();
    let view = run.ui.placed.pulled.get(&key).unwrap().clone();
    run.ui.placed.pulled = Default::default();
    run.terminal_event(Term::Key(ctrl('c')));
    assert!(!crate::palette::active(&run.ui));
    run.reply(Reply::Rendered(key, Box::new(view)), &mut None);
    assert_eq!(
        session.submissions().len(),
        1,
        "retire the late owner's cursor"
    );
    answer(&mut run, &session, &fixture, 0).await;
}
