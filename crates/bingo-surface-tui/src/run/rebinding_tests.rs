use super::*;
use crate::test_support::*;
use serde_json::json;
use std::sync::Arc;

fn fixture() -> (Run, Arc<TestSession>, crate::clock::Now) {
    fixture_with(drawing(0, ""))
}

fn fixture_with(first: View) -> (Run, Arc<TestSession>, crate::clock::Now) {
    let now = still(scene().1);
    let mut state = state();
    state.config.kernel["render"] = json!({"Pane":[{}]});
    state.signals.entry("counter".into()).or_default().insert(
        "pane".into(),
        json!({
            "slots":[{"kind":"pane","id":"counter","title":"Counter","focus":true,
            "asked":true,"closeOnEscape":true,"holdToasts":true,"rows":8,
            "view":{"request":{"component":"Pane","requestId":"counter","props":{},"viewport":{"columns":0,"rows":0}}}}]
        })
        .into(),
    );
    let session = Arc::new(TestSession::default());
    let mut run = tests::idle_in(state, session.clone(), now.instant);
    run.ui.sync_panes(run.session.tree.viewed());
    deliver_view(&mut run, now, first);
    (run, session, now)
}

fn drawing(handler: usize, value: &str) -> View {
    serde_json::from_value(json!({"kind":"Box","children":[
        {"kind":"Input","key":"name","label":"Name","value":value,
        "onInput":{"name":"counter:ui","args":{"handler":format!("name-{handler}"),"event":"input"}}}
    ]})).unwrap()
}

fn requested(run: &mut Run, now: crate::clock::Now) -> crate::pulled::Key {
    draw_tree(80, 24, &run.session.tree, &mut run.ui, now);
    let key = run.ui.shown.pulls[0].clone();
    run.ui.placed.pulled.request(&key);
    key
}

fn buttons(keyed: bool, labels: &[&str], generation: usize) -> View {
    let children = labels
        .iter()
        .map(|label| {
            json!({"kind":"Button","key":keyed.then_some(label),"label":label,
            "hotkey":label.to_lowercase(),
            "onPress":{"name":"counter:ui","args":{"handler":format!("{label}-{generation}"),"event":"press"}}})
        })
        .collect::<Vec<_>>();
    serde_json::from_value(json!({"kind":"Box","children":children})).unwrap()
}

fn deliver(run: &mut Run, now: crate::clock::Now, handler: usize, value: &str) {
    deliver_view(run, now, drawing(handler, value));
}

fn deliver_view(run: &mut Run, now: crate::clock::Now, view: View) {
    let key = requested(run, now);
    run.reply(
        Reply::Rendered(run.ui.placed.pulled.epoch(), key, Box::new(view)),
        &mut None,
    );
    draw_tree(80, 24, &run.session.tree, &mut run.ui, now);
}

fn handler(session: &TestSession, index: usize) -> serde_json::Value {
    let Input::Action { action } = &session.submitted()[index] else {
        panic!("a gesture")
    };
    action.args["handler"].clone()
}

fn invalidate(run: &mut Run, seq: u64) {
    assert!(run.fold(&frame(
        seq,
        Event::Invalidated {
            requests: vec!["counter".into()]
        }
    )));
}

fn acknowledge(run: &mut Run, session: &TestSession, index: usize) {
    run.ack(
        &session.submissions()[index].0,
        &IntentOutcome::Applied { result: json!({}) },
    );
}

#[tokio::test]
async fn rapid_pulled_edits_resolve_current_handlers_after_ack_before_rendered() {
    let (mut run, session, now) = fixture();
    for letter in "Ada".chars() {
        run.heard(Term::Key(typed(letter)));
    }
    assert_eq!(session.submitted().len(), 1);
    let mut received = Vec::new();
    for (index, value) in ["A", "Ad", "Ada"].into_iter().enumerate() {
        let Input::Action { action } = &session.submitted()[index] else {
            panic!("edit")
        };
        assert_eq!(action.args["handler"], format!("name-{index}"));
        assert_eq!(action.args["value"], value);
        received.push(value.to_owned());
        invalidate(&mut run, index as u64 + 1);
        // Exercise both an epoch bump before requesting and an already pending reply.
        if index == 1 {
            requested(&mut run, now);
        }
        acknowledge(&mut run, &session, index);
        assert_eq!(
            session.submitted().len(),
            index + 1,
            "ack cannot release an old handler"
        );
        deliver(&mut run, now, index + 1, value);
    }
    assert_eq!(received, ["A", "Ad", "Ada"]);
    assert!(run.ui.notices.is_empty(), "{:?}", run.ui.notices);
    // Discard optimistic editor state: the final drawing itself must contain Ada.
    run.ui.placed.panes = Default::default();
    run.ui.sync_panes(run.session.tree.viewed());
    let screen = draw_tree(80, 24, &run.session.tree, &mut run.ui, now);
    assert!(screen.contains("Name: Ada"), "{screen}");
}

#[tokio::test]
async fn a_missing_control_drops_its_edit_with_the_existing_stale_notice() {
    let (mut run, session, now) = fixture();
    for letter in "Ad".chars() {
        run.heard(Term::Key(typed(letter)));
    }
    invalidate(&mut run, 1);
    acknowledge(&mut run, &session, 0);
    let key = requested(&mut run, now);
    run.reply(
        Reply::Rendered(
            run.ui.placed.pulled.epoch(),
            key,
            Box::new(View::text("gone")),
        ),
        &mut None,
    );
    assert_eq!(session.submitted().len(), 1);
    assert!(run.ui.actions.action().is_none());
    assert!(run.mine.is_empty());
    assert_eq!(
        run.ui.notices[0].text,
        format!("counter: {}", bingo_sdk::view::STALE_CONTROL)
    );
    assert_eq!(run.ui.notices[0].level, Level::Warn, "the host's own level");
}

#[tokio::test]
async fn a_missing_redraw_releases_at_its_deadline_without_sleeping() {
    let (mut run, session, _) = fixture();
    for letter in "Ad".chars() {
        run.heard(Term::Key(typed(letter)));
    }
    invalidate(&mut run, 1);
    acknowledge(&mut run, &session, 0);
    let deadline = run.ui.actions.deadline().expect("bounded wait");
    assert!(run.next_draw().is_some_and(|next| next <= deadline));
    run.release_actions_at(deadline - Duration::from_nanos(1));
    assert_eq!(session.submitted().len(), 1);
    run.release_actions_at(deadline);
    assert_eq!(session.submitted().len(), 2);
    assert!(run.ui.actions.deadline().is_none());
    let Input::Action { action } = &session.submitted()[1] else {
        panic!("edit")
    };
    assert_eq!(action.args["handler"], "name-0");
    assert_eq!(action.args["value"], "Ad");
}

#[tokio::test]
async fn an_edit_typed_during_an_in_flight_redraw_uses_its_new_handler() {
    let (mut run, session, now) = fixture();
    invalidate(&mut run, 1);
    let pending = requested(&mut run, now);
    for letter in "Ada".chars() {
        run.heard(Term::Key(typed(letter)));
    }
    assert!(
        session.submitted().is_empty(),
        "the old drawing is still visible"
    );
    run.reply(
        Reply::Rendered(
            run.ui.placed.pulled.epoch(),
            pending,
            Box::new(drawing(1, "")),
        ),
        &mut None,
    );
    for (index, value) in ["A", "Ad", "Ada"].into_iter().enumerate() {
        let Input::Action { action } = &session.submitted()[index] else {
            panic!("edit")
        };
        assert_eq!(action.args["handler"], format!("name-{}", index + 1));
        assert_eq!(action.args["value"], value);
        invalidate(&mut run, index as u64 + 2);
        acknowledge(&mut run, &session, index);
        assert_eq!(session.submitted().len(), index + 1);
        deliver(&mut run, now, index + 2, value);
    }
    assert!(run.ui.notices.is_empty());
}

#[tokio::test]
async fn queued_acknowledgements_release_edits_without_waiting_for_a_turn() {
    for redraw_pending in [false, true] {
        let (mut run, session, _) = fixture();
        for letter in "Ad".chars() {
            run.heard(Term::Key(typed(letter)));
        }
        if redraw_pending {
            invalidate(&mut run, 1);
        }
        run.ack(
            &session.submissions()[0].0,
            &IntentOutcome::Queued { position: 1 },
        );
        if redraw_pending {
            assert_eq!(session.submitted().len(), 1);
            let deadline = run
                .ui
                .actions
                .deadline()
                .expect("only a bounded redraw wait");
            run.release_actions_at(deadline);
        }
        assert_eq!(session.submitted().len(), 2);
        assert!(run.ui.actions.deadline().is_none());
        run.ack(
            &session.submissions()[1].0,
            &IntentOutcome::Queued { position: 2 },
        );
        assert!(run.ui.actions.action().is_none());
        assert!(run.mine.is_empty());
    }
}

#[tokio::test]
async fn another_sites_pending_redraw_does_not_hold_this_sites_edits() {
    let (mut run, session, now) = fixture();
    let mut other = requested(&mut run, now);
    other.request.request_id = "another-pane".into();
    run.ui.placed.pulled.request(&other);
    run.fold(&frame(
        1,
        Event::Invalidated {
            requests: vec!["another-pane".into()],
        },
    ));
    for letter in "Ad".chars() {
        run.heard(Term::Key(typed(letter)));
    }
    assert_eq!(session.submitted().len(), 1);
    acknowledge(&mut run, &session, 0);
    assert_eq!(session.submitted().len(), 2);
    assert!(run.ui.actions.deadline().is_none());
}

#[tokio::test]
async fn an_in_flight_action_does_not_defer_pane_pulls() {
    let (mut run, session, now) = fixture();
    run.heard(Term::Key(typed('A')));
    assert_eq!(session.submitted().len(), 1);
    assert!(run.ui.actions.action().is_some());
    invalidate(&mut run, 1);
    draw_tree(80, 24, &run.session.tree, &mut run.ui, now);
    let key = run.ui.shown.pulls[0].clone();
    pulling::request(&mut run, std::slice::from_ref(&key));
    assert!(
        !run.ui.placed.pulled.request(&key),
        "the redraw was already requested while the action was in flight"
    );
}

/// `a` is in flight while `b` waits on a redraw that adds `X` in front.
fn press_b_across_a_shift(keyed: bool) -> (Run, Arc<TestSession>) {
    let (mut run, session, now) = fixture_with(buttons(keyed, &["A", "B"], 0));
    run.heard(Term::Key(typed('a')));
    run.heard(Term::Key(typed('b')));
    assert_eq!(session.submitted().len(), 1);
    invalidate(&mut run, 1);
    acknowledge(&mut run, &session, 0);
    assert_eq!(session.submitted().len(), 1, "held for the redraw");
    deliver_view(&mut run, now, buttons(keyed, &["X", "A", "B"], 1));
    (run, session)
}

#[tokio::test]
async fn an_unkeyed_button_shifted_by_a_redraw_is_stale_not_its_neighbour() {
    let (run, session) = press_b_across_a_shift(false);
    assert_eq!(session.submitted().len(), 1, "A-1 now sits where B was");
    assert_eq!(
        run.ui.notices[0].text,
        format!("counter: {}", bingo_sdk::view::STALE_CONTROL)
    );
    assert_eq!(run.ui.notices[0].level, Level::Warn);
}

#[tokio::test]
async fn a_keyed_button_moved_by_a_redraw_takes_its_new_handler() {
    let (run, session) = press_b_across_a_shift(true);
    assert_eq!(handler(&session, 1), "B-1");
    assert!(run.ui.notices.is_empty(), "{:?}", run.ui.notices);
}

#[tokio::test]
async fn a_held_press_keeps_its_pending_mark_in_the_drawing() {
    let (mut run, session, now) = fixture_with(buttons(true, &["A"], 0));
    let waiting = format!("A{}", crate::theme::ellipsis());
    run.heard(Term::Key(typed('a')));
    run.heard(Term::Key(typed('a')));
    invalidate(&mut run, 1);
    acknowledge(&mut run, &session, 0);
    assert_eq!(session.submitted().len(), 1);
    assert!(run.ui.actions.deadline().is_some(), "held for the redraw");
    let screen = draw_tree(80, 24, &run.session.tree, &mut run.ui, now);
    assert!(screen.contains(&waiting), "{screen}");
}

#[tokio::test(start_paused = true)]
async fn a_held_edit_expires_on_the_runs_clock() {
    let (mut run, session, _) = fixture();
    for letter in "Ad".chars() {
        run.heard(Term::Key(typed(letter)));
    }
    invalidate(&mut run, 1);
    let start = tokio::time::Instant::now().into_std();
    acknowledge(&mut run, &session, 0);
    assert_eq!(
        run.ui.actions.deadline(),
        Some(start + crate::actions::REDRAW_WAIT)
    );
    tokio::time::advance(crate::actions::REDRAW_WAIT - Duration::from_millis(1)).await;
    run.release_actions();
    assert_eq!(session.submitted().len(), 1);
    tokio::time::advance(Duration::from_millis(1)).await;
    run.release_actions();
    assert_eq!(session.submitted().len(), 2);
    assert_eq!(handler(&session, 1), "name-0");
}

#[tokio::test]
async fn a_rebuilt_memo_holds_an_edit_until_its_site_redraws() {
    let (mut run, session, now) = fixture();
    for letter in "Ad".chars() {
        run.heard(Term::Key(typed(letter)));
    }
    run.catalog_changed(CatalogKind::Commands);
    acknowledge(&mut run, &session, 0);
    assert_eq!(session.submitted().len(), 1, "the site is still placed");
    assert!(run.ui.actions.deadline().is_some());
    assert!(run.ui.notices.is_empty(), "{:?}", run.ui.notices);
    deliver(&mut run, now, 1, "Ad");
    assert_eq!(handler(&session, 1), "name-1");
    assert!(run.ui.notices.is_empty(), "{:?}", run.ui.notices);
}
