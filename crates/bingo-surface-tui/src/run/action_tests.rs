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
    state.signals.entry("counter".into()).or_default().insert(
        "pane".into(),
        bingo_sdk::slot::with(json!({}), [slot]).into(),
    );
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
        let Some((intent, Input::Action { action, .. })) = next else {
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
    run.interrupt(run.session.tree.view().clone());
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
        matches!(&next.submitted()[..], [Input::Action { action, .. }] if action.args["value"] == "B")
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
            (
                bingo_sdk::Action {
                    name: "counter:ui".into(),
                    args: json!({"value":session}),
                },
                Some(bingo_sdk::Origin::surface(crate::SURFACE_ID)),
            ),
            None,
        );
    }
    actions.release(Instant::now(), |_, _| unreachable!());
    assert_eq!(first.submitted().len(), 1);
    assert!(second.submitted().is_empty());
    assert_eq!(
        actions.discard(&[SessionId::from_raw("first")]),
        [IntentId::from_raw("first")]
    );
    actions.acknowledge(&IntentId::from_raw("first"));
    actions.release(Instant::now(), |_, _| unreachable!());
    assert_eq!(first.submitted().len(), 1);
    assert!(
        matches!(&second.submitted()[..], [Input::Action { action, .. }] if action.args["value"] == "second")
    );
}

#[tokio::test]
async fn escape_on_an_ask_discards_its_owners_waiting_actions_only() {
    let now = still(scene().1);
    let root = Arc::new(TestSession::default());
    let child = Arc::new(TestSession::default());
    let mut run = tests::idle_in(counter(""), root.clone(), now.instant);
    let mut screen = Recorder::default();
    run.paint(&mut screen, Wake::Frame, now).unwrap();
    for letter in "Ada".chars() {
        run.heard(Term::Key(typed(letter)));
    }
    let first = root.submissions()[0].0.clone();
    run.session
        .tree
        .fold
        .apply(&child_frame(1, announced("child")));
    run.session
        .handles
        .insert(child_id(), SessionHandle(child.clone()));
    run.session.tree.show(&child_id());
    run.submit(crate::acted(bingo_sdk::Action {
        name: "counter:ui".into(),
        args: json!({"value":"child edit"}),
    }));
    run.frame(&frame(1, opened(permission(None, None))), &mut screen)
        .unwrap();
    run.session.tree.show(&state().summary.id);
    run.heard(Term::Key(key(crossterm::event::KeyCode::Esc)));
    assert_eq!(root.interrupts(), 1);
    assert_eq!(child.interrupts(), 0);
    assert_eq!(run.ui.actions.action().unwrap().args["value"], "child edit");
    assert_eq!(child.submitted().len(), 1, "unrelated actions still run");
    run.ack(&first, &IntentOutcome::Applied { result: json!({}) });
    assert_eq!(
        root.submitted().len(),
        1,
        "late ack must not replay discarded edits"
    );
    run.ack(
        &child.submissions()[0].0,
        &IntentOutcome::Applied { result: json!({}) },
    );
    assert!(run.ui.actions.action().is_none());
}

#[tokio::test]
async fn switching_discards_only_an_empty_bare_resume_bootstrap() {
    for (bootstrap, messages, title, expected) in [
        (true, 0, None, "delete"),
        (true, 1, None, "close"),
        (false, 0, None, "close"),
        (true, 0, Some("named"), "close"),
    ] {
        let now = scene().1;
        let mut snapshot = state();
        snapshot.summary.history.messages = Some(messages);
        snapshot.summary.title = title.map(str::to_owned);
        let mut run = tests::idle_in(
            snapshot.clone(),
            Arc::new(TestSession::default()),
            now.instant,
        );
        run.session.bootstrap = bootstrap;
        let host = bingo_sdk::testing::TestHost::default();
        let (sent, mut received) = tokio::sync::mpsc::unbounded_channel();
        let deleting = sent.clone();
        host.on_delete(move |_| {
            let sent = deleting.clone();
            async move {
                sent.send("delete").unwrap();
                Ok(())
            }
        });
        host.on_close(move |_, _| {
            let sent = sent.clone();
            async move {
                sent.send("close").unwrap();
                Ok(())
            }
        });
        run.host = host.handle();
        snapshot.summary.id = SessionId::from_raw("replacement");
        run.attach(
            Attachment {
                session: snapshot.summary.id.clone(),
                snapshot,
                history: None,
                events: Box::pin(futures::stream::empty()),
                handle: SessionHandle(Arc::new(TestSession::default())),
            },
            &mut None,
        );
        assert_eq!(
            tokio::time::timeout(std::time::Duration::from_secs(10), received.recv())
                .await
                .unwrap()
                .unwrap(),
            expected
        );
    }
}

#[tokio::test]
async fn queued_actions_do_not_block_first_pane_or_status_drawings() {
    use bingo_sdk::render::{Component, RenderRequest, SurfaceName, Viewport};
    let now = still(scene().1);
    let session = Arc::new(TestSession::default());
    let mut run = tests::idle_in(counter(""), session.clone(), now.instant);
    run.paint(&mut Recorder::default(), Wake::Frame, now)
        .unwrap();
    for letter in "Ada".chars() {
        run.heard(Term::Key(typed(letter)));
    }
    let host = bingo_sdk::testing::TestHost::default();
    host.on_render(|_, _| async { Ok(bingo_sdk::View::text("new callbacks")) });
    run.host = host.handle();
    let request = |component, id: &str| {
        crate::pulled::Key::new(
            run.session.tree.viewed(),
            &RenderRequest {
                component,
                request_id: id.into(),
                surface: SurfaceName::Terminal,
                props: json!({}),
                viewport: Viewport {
                    place: None,
                    columns: 80,
                    rows: 8,
                },
            },
            80,
            8,
        )
    };
    let pane = request(Component::Pane, "counter");
    let status = request(Component::Status, "status");
    assert!(pulling::request(&mut run, &[status.clone(), pane.clone()]));
    assert!(run.ui.placed.pulled.get(&status).is_some());
    assert!(run.ui.actions.action().is_some());
    assert_eq!(
        run.ui.placed.pulled.get(&pane).unwrap().fold(),
        "new callbacks"
    );
}

#[tokio::test]
async fn a_surface_action_prefills_the_composer_without_submitting() {
    let now = still(scene().1);
    let session = Arc::new(TestSession::default());
    let mut run = tests::idle_in(state(), session.clone(), now.instant);
    for text in ["/rename ", "/permission ", "/wake off"] {
        run.ui.composer.set("");
        surface::receipt(
            &mut run,
            &json!({"surface":{"name":"Composer","args":{"text":text}}}),
        );
        assert_eq!(run.ui.composer.text(), text);
        assert!(session.submissions().is_empty());
    }
}

#[tokio::test]
async fn a_composer_action_preserves_an_existing_draft_and_explains_why() {
    let now = still(scene().1);
    let session = Arc::new(TestSession::default());
    for text in ["/rename ", "/permission ", "/wake off"] {
        let mut run = tests::idle_in(state(), session.clone(), now.instant);
        run.ui.composer.set("my unfinished draft");
        surface::receipt(
            &mut run,
            &json!({"surface":{"name":"Composer","args":{"text":text}}}),
        );
        assert_eq!(run.ui.composer.text(), "my unfinished draft");
        assert_eq!(
            run.ui.notice().unwrap().plain().unwrap().text,
            format!("composer has a draft; clear it to use {}", text.trim())
        );
        assert!(session.submissions().is_empty());
    }
}

#[tokio::test]
async fn only_the_key_acknowledgement_creates_a_mode_notice() {
    let now = scene().1;
    let session = Arc::new(TestSession::default());
    let mut run = tests::idle_in(state(), session.clone(), now.instant);
    run.cycle_mode(Input::text("/permission plan", crate::origin()));
    run.run_line("/permission plan".into());
    let submitted = session.submissions();
    let receipt = IntentOutcome::Applied {
        result: json!({"message":"permission mode: plan"}),
    };
    run.ack(&submitted[1].0, &receipt);
    assert_eq!(
        run.ui.notice().unwrap().plain().unwrap().text,
        "permission mode: plan"
    );
    run.ack(&IntentId::from_raw("other-client"), &receipt);
    assert!(run.ui.notice().unwrap().plain().is_some());
    run.ack(&submitted[0].0, &receipt);
    assert!(matches!(
        run.ui.notice().unwrap().kind,
        crate::ui::NoticeKind::Mode
    ));
    run.ack(&submitted[1].0, &receipt);
    assert!(matches!(
        run.ui.notice().unwrap().kind,
        crate::ui::NoticeKind::Mode
    ));
}

#[tokio::test]
async fn a_refused_mode_key_names_the_command_it_submitted() {
    let now = scene().1;
    let session = Arc::new(TestSession::default());
    let mut run = tests::idle_in(state(), session.clone(), now.instant);
    run.cycle_mode(Input::text("/permission plan", crate::origin()));
    run.ack(
        &session.submissions()[0].0,
        &IntentOutcome::Rejected {
            error: KernelError::new(bingo_sdk::ErrorCode::InvalidInput, "refused"),
        },
    );
    assert_eq!(
        run.ui.notice().unwrap().plain().unwrap().about.as_deref(),
        Some("/permission plan")
    );
}
