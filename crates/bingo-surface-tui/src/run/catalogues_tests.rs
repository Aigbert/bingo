use super::*;
use crate::{doubles::TestSession, test_support::*};
use bingo_sdk::{Catalog, testing::TestHost};
use std::sync::Arc;

fn answer(kind: CatalogKind) -> Catalog {
    let entries = if kind == CatalogKind::Commands {
        vec![
            serde_json::from_value(serde_json::json!({"id":"review","label":"review","meta":{
                "name":"review","hint":"review","family":"terminal","args":{"kind":"none"},
                "instant":true,"opens":true,"keys":["ctrl+n"]
            }}))
            .unwrap(),
        ]
    } else {
        vec![]
    };
    Catalog { kind, entries }
}

#[tokio::test(start_paused = true)]
async fn the_first_key_sees_a_prompt_command_catalogue() {
    let mut run = super::super::tests::idle(scene().1.instant);
    let host = TestHost::default();
    host.on_catalog(|kind| async move { Ok(answer(kind)) });
    run.host = host.handle();
    fetch_all(&mut run).await;
    assert_eq!(
        crate::keys::commands::line(&run.ui, run.session.tree.viewed(), ctrl('n')).as_deref(),
        Some("/review")
    );
    assert!(!run.typeahead.catalogue_pending);
}

#[tokio::test(start_paused = true)]
async fn a_slow_catalogue_releases_first_paint_and_keeps_the_first_shortcut() {
    let session = Arc::new(TestSession::default());
    let mut run = super::super::tests::idle_in(state(), session.clone(), scene().1.instant);
    let host = TestHost::default();
    let release = Arc::new(tokio::sync::Notify::new());
    let gate = release.clone();
    host.on_catalog(move |kind| {
        let gate = gate.clone();
        async move {
            if kind == CatalogKind::Commands {
                gate.notified().await;
            }
            Ok(answer(kind))
        }
    });
    run.host = host.handle();
    let (tx, mut replies) = mpsc::channel(16);
    run.replies = tx;
    let start = tokio::time::Instant::now();
    fetch_all(&mut run).await;
    assert_eq!(start.elapsed(), INITIAL_WAIT);
    assert!(draw_tree(80, 24, &run.session.tree, &mut run.ui, scene().1).contains("ask anything"));
    run.terminal_event(Term::Key(typed('a')));
    assert_eq!(run.ui.composer.text(), "a");
    run.terminal_event(Term::Key(ctrl('n')));
    assert_eq!(run.typeahead.len(), 1);
    assert!(session.submitted().is_empty());
    release.notify_one();
    loop {
        let reply = replies.recv().await.unwrap();
        let ready = matches!(reply, Reply::Commands(_));
        run.reply(reply, &mut None);
        if ready {
            break;
        }
    }
    assert!(matches!(&session.submitted()[..], [Input::Text { text, .. }] if text == "/review"));
    assert!(run.typeahead.is_empty());
}

#[tokio::test(start_paused = true)]
async fn a_catalogue_error_keeps_startup_and_typing_available() {
    let mut run = super::super::tests::idle(scene().1.instant);
    let host = TestHost::default();
    host.on_catalog(|_| async { Err(KernelError::new(bingo_sdk::ErrorCode::Internal, "offline")) });
    run.host = host.handle();
    fetch_all(&mut run).await;
    assert!(!run.typeahead.catalogue_pending);
    assert!(
        run.ui
            .notices
            .iter()
            .any(|notice| notice.text.contains("offline"))
    );
    run.terminal_event(Term::Key(typed('a')));
    assert_eq!(run.ui.composer.text(), "a");
}

#[tokio::test(start_paused = true)]
async fn a_background_error_releases_held_input_and_allows_a_later_refresh() {
    let mut run = super::super::tests::idle(scene().1.instant);
    run.typeahead.catalogue_pending = true;
    run.terminal_event(Term::Key(ctrl('n')));
    run.terminal_event(Term::Key(typed('a')));
    run.reply(
        Reply::Commands(Err(KernelError::new(
            bingo_sdk::ErrorCode::Internal,
            "offline",
        ))),
        &mut None,
    );
    assert!(run.typeahead.is_empty());
    assert_eq!(run.ui.composer.text(), "a");
    run.reply(
        Reply::Commands(Ok(commands::specs_from(&answer(CatalogKind::Commands)))),
        &mut None,
    );
    assert_eq!(
        crate::keys::commands::line(&run.ui, run.session.tree.viewed(), ctrl('n')).as_deref(),
        Some("/review")
    );
}

#[tokio::test(start_paused = true)]
async fn a_stalled_initial_catalogue_fails_at_its_overall_deadline() {
    let mut run = super::super::tests::idle(scene().1.instant);
    let host = TestHost::default();
    host.on_catalog(|kind| async move {
        if kind == CatalogKind::Commands {
            std::future::pending::<()>().await;
        }
        Ok(answer(kind))
    });
    run.host = host.handle();
    let (tx, mut replies) = mpsc::channel(16);
    run.replies = tx;
    let start = tokio::time::Instant::now();
    fetch_all(&mut run).await;
    run.terminal_event(Term::Key(ctrl('n')));
    run.terminal_event(Term::Key(typed('a')));
    assert_eq!(run.ui.composer.text(), "a");
    assert_eq!(run.typeahead.len(), 1);
    loop {
        let reply = replies.recv().await.unwrap();
        let ready = matches!(reply, Reply::Commands(_));
        run.reply(reply, &mut None);
        if ready {
            break;
        }
    }
    assert_eq!(start.elapsed(), COMMAND_WAIT);
    assert!(!run.typeahead.catalogue_pending);
    assert!(run.typeahead.is_empty());
    assert!(run.ui.notices.iter().any(|n| n.text.contains("timed out")));
}

#[tokio::test]
async fn only_catalogue_holds_let_plain_keys_and_exit_pass() {
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
    for key in [
        ctrl('n'),
        KeyEvent::new(KeyCode::Char('n'), KeyModifiers::ALT),
        KeyEvent::new(KeyCode::F(2), KeyModifiers::NONE),
    ] {
        let mut run = super::super::tests::idle(scene().1.instant);
        run.typeahead.catalogue_pending = true;
        run.terminal_event(Term::Key(key));
        run.terminal_event(Term::Key(typed('a')));
        assert_eq!(run.ui.composer.text(), "a");
        assert_eq!(run.typeahead.len(), 1);
        run.ui.composer.set("");
        run.terminal_event(Term::Key(ctrl('d')));
        assert!(run.exit.is_some());
        let mut run = super::super::tests::idle(scene().1.instant);
        run.typeahead.catalogue_pending = true;
        run.terminal_event(Term::Key(key));
        typeahead::command_sent(&mut run, IntentId::mint());
        run.terminal_event(Term::Key(typed('a')));
        assert_eq!(run.typeahead.len(), 2, "opening still preserves order");
    }
}
