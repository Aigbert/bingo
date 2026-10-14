use super::*;
use crate::test_support::*;
use serde_json::json;
use std::{collections::HashSet, sync::Arc};

fn counter(name: &str) -> SessionState {
    let mut state = state();
    let slot = serde_json::from_value(json!({
        "kind":"pane", "id":"counter", "title":"Counter", "focus":true,
        "asked":true, "closeOnEscape":true, "holdToasts":true, "rows":8,
        "view":{"kind":"Input", "label":"Name", "value":name,
            "onInput":{"name":"counter:ui", "args":{"handler":"name", "event":"input"}}}
    }))
    .unwrap();
    state
        .signals
        .entry("counter".into())
        .or_default()
        .insert("pane".into(), bingo_sdk::slot::with(json!({}), [slot]));
    state
}

fn finish_newest(run: &mut Run, session: &TestSession) -> (String, Vec<String>) {
    let mut finished = HashSet::new();
    let mut saved = String::new();
    let mut trace = Vec::new();
    loop {
        let next = session
            .submissions()
            .into_iter()
            .rev()
            .find(|(intent, _)| !finished.contains(intent));
        let Some((intent, Input::Action { action })) = next else {
            break;
        };
        if action.args["event"] == "input" {
            saved = action.args["value"].as_str().unwrap().to_owned();
            trace.push(saved.clone());
        } else {
            trace.push(format!("close: {saved}"));
        }
        finished.insert(intent.clone());
        run.ack(&intent, &IntentOutcome::Applied { result: json!({}) });
    }
    (saved, trace)
}

#[tokio::test]
async fn pane_edits_persist_in_key_order_even_if_newer_actions_finish_first() {
    let now = still(scene().1);
    let session = Arc::new(TestSession::default());
    let mut run = tests::idle_in(counter(""), session.clone(), now.instant);
    let mut screen = Recorder::default();
    run.paint(&mut screen, Wake::Frame, now).unwrap();
    for letter in "Ada".chars() {
        run.heard(Term::Key(typed(letter)));
        // An older render may arrive between keys while its action is in flight.
        run.session.tree = solo(&counter("Ad"));
        run.paint(&mut screen, Wake::Frame, now).unwrap();
    }
    assert!(screen.last().contains("Name: Ada"), "{}", screen.last());
    let (saved, trace) = finish_newest(&mut run, &session);
    assert_eq!(saved, "Ada", "completed input values: {trace:?}");
    run.ui.placed.panes = Default::default();
    run.session.tree = solo(&counter(&saved));
    run.paint(&mut screen, Wake::Frame, now).unwrap();
    assert!(screen.last().contains("Name: Ada"), "{}", screen.last());
}

#[tokio::test]
async fn closing_a_pane_follows_its_last_edit() {
    let now = still(scene().1);
    let session = Arc::new(TestSession::default());
    let mut run = tests::idle_in(counter(""), session.clone(), now.instant);
    let mut screen = Recorder::default();
    run.paint(&mut screen, Wake::Frame, now).unwrap();
    for letter in "Ada".chars() {
        run.heard(Term::Key(typed(letter)));
    }
    run.heard(Term::Key(key(crossterm::event::KeyCode::Esc)));
    let (_, trace) = finish_newest(&mut run, &session);
    assert_eq!(trace, ["A", "Ad", "Ada", "close: Ada"]);
}

#[test]
fn unrelated_or_duplicate_acks_cannot_release_an_edit_but_rejection_does() {
    let now = scene().1;
    let session = Arc::new(TestSession::default());
    let mut run = tests::idle_in(counter(""), session.clone(), now.instant);
    let mut screen = Recorder::default();
    run.paint(&mut screen, Wake::Frame, now).unwrap();
    for letter in "Ad".chars() {
        run.heard(Term::Key(typed(letter)));
    }
    let rejected = IntentOutcome::Rejected {
        error: KernelError::new(bingo_sdk::ErrorCode::InvalidInput, "fixture refusal"),
    };
    run.ack(&IntentId::from_raw("unrelated"), &rejected);
    assert_eq!(session.submitted().len(), 1);
    let first = session.submissions()[0].0.clone();
    run.ack(&first, &rejected);
    run.ack(&first, &rejected);
    assert_eq!(session.submitted().len(), 2);
    assert_eq!(run.ui.actions.action().unwrap().args["value"], "Ad");
}

#[test]
fn an_interrupt_discards_waiting_edits_instead_of_replaying_them_on_ack() {
    let now = scene().1;
    let session = Arc::new(TestSession::default());
    let mut run = tests::idle_in(counter(""), session.clone(), now.instant);
    let mut screen = Recorder::default();
    run.paint(&mut screen, Wake::Frame, now).unwrap();
    for letter in "Ada".chars() {
        run.heard(Term::Key(typed(letter)));
    }
    let first = session.submissions()[0].0.clone();
    run.interrupt();
    run.ack(&first, &IntentOutcome::Applied { result: json!({}) });
    assert_eq!(session.submitted().len(), 1);
    assert_eq!(session.interrupts(), 1);
    assert!(run.ui.actions.action().is_none());
}

#[tokio::test]
async fn replacing_the_attachment_discards_its_waiting_edits() {
    let now = scene().1;
    let old = Arc::new(TestSession::default());
    let mut run = tests::idle_in(counter(""), old.clone(), now.instant);
    let mut screen = Recorder::default();
    run.paint(&mut screen, Wake::Frame, now).unwrap();
    for letter in "Ada".chars() {
        run.heard(Term::Key(typed(letter)));
    }
    let next = Arc::new(TestSession::default());
    let mut snapshot = counter("");
    snapshot.summary.id = SessionId::from_raw("replacement");
    run.attach(
        Attachment {
            session: snapshot.summary.id.clone(),
            snapshot,
            history: None,
            events: Box::pin(futures::stream::empty()),
            handle: SessionHandle(next.clone()),
        },
        &mut None,
    );
    run.ack(
        &old.submissions()[0].0,
        &IntentOutcome::Applied { result: json!({}) },
    );
    run.paint(&mut screen, Wake::Frame, now).unwrap();
    run.heard(Term::Key(typed('B')));
    assert_eq!(old.submitted().len(), 1);
    assert!(
        matches!(&next.submitted()[..], [Input::Action { action }] if action.args["value"] == "B")
    );
}

#[test]
fn removing_a_session_releases_the_next_action_to_its_original_mailbox() {
    let first = Arc::new(TestSession::default());
    let second = Arc::new(TestSession::default());
    let mut actions = crate::actions::Queue::default();
    for (session, handle) in [("first", first.clone()), ("second", second.clone())] {
        actions.push(
            SessionId::from_raw(session),
            SessionHandle(handle),
            IntentId::from_raw(session),
            bingo_sdk::Action {
                name: "counter:ui".into(),
                args: json!({"value":session}),
            },
        );
    }
    assert_eq!(first.submitted().len(), 1);
    assert!(second.submitted().is_empty());
    assert_eq!(
        actions.discard(&[SessionId::from_raw("first")]),
        [IntentId::from_raw("first")]
    );
    actions.acknowledge(&IntentId::from_raw("first"));
    assert_eq!(first.submitted().len(), 1);
    assert!(
        matches!(&second.submitted()[..], [Input::Action { action }] if action.args["value"] == "second")
    );
}
