use super::*;
use crate::test_support::*;
use serde_json::json;
#[tokio::test]
async fn result_heads_use_only_the_original_command_and_escape_keeps_the_draft() {
    for (line, head) in [(Some("/example secret argument"), "/example"), (None, "")] {
        let mut run = tests::idle(Instant::now());
        run.ui.composer.set("my draft");
        let intent = run.mint(Submission::Ordinary(line.map(str::to_owned)));
        run.ack(
            &intent,
            &IntentOutcome::Applied {
                result: json!({"view":View::text("one\ntwo")}),
            },
        );
        assert!(matches!(run.ui.focus.open(), Open::Result { head:got, .. } if got == head));
        let now = still(scene().1);
        let text = draw_tree(80, 24, &run.session.tree, &mut run.ui, now);
        assert!(text.contains("one") && text.contains("two"));
        assert!(!text.contains("secret argument"));
        assert!(!text.contains("my draft"));
        let effects = input::on_key(
            &mut run.ui,
            &run.session.tree,
            key(crossterm::event::KeyCode::Esc),
            now,
        );
        assert!(effects.is_empty());
        assert!(!run.ui.focus.showing());
        assert_eq!(run.ui.composer.text(), "my draft");
    }
}

#[test]
fn result_scrolling_counts_wrapped_rows_and_preserves_the_view() {
    let now = still(scene().1);
    let mut run = tests::idle(now.instant);
    let view = bingo_sdk::pick::text(
        (0..40).map(|i| format!("row {i}\n")).collect::<String>(),
        "warn",
    );
    run.ui.focus.show(
        Open::Result {
            head: "/example".into(),
            view: view.clone(),
            at: 0,
        },
        now.instant,
    );
    let before = draw_tree(80, 24, &run.session.tree, &mut run.ui, now);
    assert!(before.contains("row 0"));
    for _ in 0..3 {
        crate::dock::result::keys(&mut run.ui, key(crossterm::event::KeyCode::PageDown), now);
    }
    let after = draw_tree(80, 24, &run.session.tree, &mut run.ui, now);
    assert!(!after.contains("row 0"));
    assert!(after.contains("row 24"));
    assert!(
        matches!(run.ui.focus.open(), Open::Result { view: kept, at: 24, .. } if kept == &view)
    );
}

#[test]
fn rewind_counts_belong_only_to_the_opening_that_requested_them() {
    let now = still(scene().1);
    let mut run = tests::idle(now.instant);
    let first = crate::rewind::Rewind::default();
    let request = first.request.clone();
    run.ui.focus.show(Open::Rewind(first), now.instant);
    run.ui.focus.close(now.instant);
    let second = crate::rewind::Rewind::default();
    let current = second.request.clone();
    run.ui.focus.show(Open::Rewind(second), now.instant);
    let turn = bingo_sdk::TurnId::from_raw("old-turn");
    let session = run.session.tree.view().clone();
    run.reply(
        Reply::RewindFiles(super::pulling::TurnFiles {
            request,
            session: session.clone(),
            turn: turn.clone(),
            touched: None,
        }),
        &mut None,
    );
    let Open::Rewind(card) = run.ui.focus.open() else {
        panic!("rewind")
    };
    assert!(card.touched.is_empty());
    run.reply(
        Reply::RewindFiles(super::pulling::TurnFiles {
            request: current,
            session,
            turn: turn.clone(),
            touched: None,
        }),
        &mut None,
    );
    let Open::Rewind(card) = run.ui.focus.open() else {
        panic!("rewind")
    };
    assert_eq!(card.touched.get(&turn), Some(&None));
    let view = crate::rewind::sheet(
        &bingo_sdk::transcript::turns(&run.session.tree.viewed().transcript),
        card,
        80,
        20,
    );
    assert!(!view.fold().contains("no files"));
}
