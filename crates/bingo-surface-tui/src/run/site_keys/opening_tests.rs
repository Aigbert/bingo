use super::*;
use crate::{doubles::TestHost, test_support::*};
use serde_json::json;

fn pane_run() -> (
    Run,
    std::sync::Arc<crate::doubles::TestSession>,
    mpsc::Receiver<Reply>,
) {
    let mut state = state();
    let pane: bingo_sdk::slot::Slot = serde_json::from_value(json!({
        "kind":"pane", "id":"picker", "title":"", "focus":true, "asked":true,
        "closeOnEscape":true, "holdToasts":true,
        "rows":6, "view":View::text("picker")
    }))
    .unwrap();
    state.signals.entry("fixture".into()).or_default().insert(
        "picker".into(),
        bingo_sdk::slot::with(json!({}), [pane]).into(),
    );
    let (host, session) = TestHost::with(vec![]);
    let mut run = super::super::tests::idle_in(state, session.clone(), Instant::now());
    run.host = host;
    run.session.handles.clear();
    run.ui.focus = crate::focus::Focus::Pane("ses_1/fixture/picker".into());
    let (sender, receiver) = mpsc::channel(4);
    run.replies = sender;
    (run, session, receiver)
}

fn send_key(run: &mut Run) {
    send(
        run,
        bingo_sdk::Action {
            name: "fixture:ui".into(),
            args: json!({"handler":"picker", "value":{"key":"x"}}),
        },
        typed('x'),
    );
    assert_eq!(run.site_keys.len(), 1, "a key waits for the missing handle");
}

#[tokio::test]
async fn a_pane_key_waits_for_its_handle_and_reaches_its_site() {
    let (mut run, session, mut replies) = pane_run();
    send_key(&mut run);
    assert!(session.submissions().is_empty());
    run.reply(replies.recv().await.unwrap(), &mut None);
    let submitted = session.submissions();
    assert_eq!(submitted.len(), 1, "a pane key is not an interaction ID");
    assert!(matches!(&submitted[0].1, Input::Action { action, .. }
        if action.args["value"] == json!({"key":"x"})));
    assert_eq!(run.site_keys.len(), 1, "the site still owes its answer");
}

#[tokio::test]
async fn a_directory_request_survives_opening_its_handle() {
    let (mut run, session, mut replies) = pane_run();
    let now = run.detection.now();
    crate::sessions::open(&mut run.ui, now);
    send_key(&mut run);
    run.reply(replies.recv().await.unwrap(), &mut None);
    assert_eq!(
        session.submissions().len(),
        1,
        "a request ID is a live site"
    );
}

/// A key the open ask answers is this surface's own press, though a pane a
/// plugin drew still holds focus under the dialog.
#[tokio::test]
async fn a_key_routed_past_a_focused_pane_is_signed_by_its_own_site() {
    let (mut run, session, mut replies) = pane_run();
    run.session.tree.fold.apply(&frame(
        1,
        crate::test_support::opened(permission(None, None)),
    ));
    send(
        &mut run,
        bingo_sdk::Action {
            name: "ask:ui".into(),
            args: json!({"value":{"key":"x"}}),
        },
        typed('x'),
    );
    run.reply(replies.recv().await.unwrap(), &mut None);
    let submitted = session.submissions();
    assert!(
        matches!(&submitted[..], [(_, Input::Action { origin, .. })]
            if origin == &Some(bingo_sdk::Origin::surface(crate::SURFACE_ID))),
        "{submitted:?}"
    );
}

#[tokio::test]
async fn a_pane_closing_before_its_handle_returns_readdresses_in_typed_order() {
    delayed_handoff(false).await;
}

#[tokio::test]
async fn a_pane_closing_before_its_handle_returns_never_answers_an_ask() {
    delayed_handoff(true).await;
}

async fn delayed_handoff(ask: bool) {
    let (mut run, session, mut replies) = pane_run();
    if ask {
        run.session.tree.fold.apply(&frame(
            1,
            crate::test_support::opened(permission(None, None)),
        ));
        run.ui.dialog.aside = true;
    }
    send_key(&mut run);
    typeahead::queue(&mut run, typed('y'));
    run.session.tree.fold.apply(&frame(
        2,
        Event::Signal {
            generation: None,
            plugin: "fixture".into(),
            kind: "picker".into(),
            payload: json!({}),
        },
    ));
    if ask {
        run.ui.dialog.aside = false;
    }
    run.reply(replies.recv().await.unwrap(), &mut None);
    assert!(
        session.submissions().is_empty(),
        "nothing reaches the closed site"
    );
    assert!(session.answers().is_empty());
    assert!(run.typeahead.is_empty());
    assert!(run.site_keys.is_empty());
    assert_eq!(run.ui.composer.text(), if ask { "" } else { "xy" });
    if ask {
        assert_eq!(
            run.ui.notice().expect("a notice").plain().unwrap().text,
            typeahead::kept_out_notice(2)
        );
    }
}
