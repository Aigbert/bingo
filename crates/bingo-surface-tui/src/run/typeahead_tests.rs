//! Keys typed ahead of the site that reads them are never lost (M180).
use super::*;
use crate::{doubles::TestSession, test_support::*};
use bingo_sdk::{Action, render::Component};
use crossterm::event::KeyCode;
use serde_json::json;
use std::sync::Arc;

/// Answers the open site's drawing with a view that sends its keys back.
fn pull(run: &mut Run, component: Component, now: crate::clock::Now) {
    draw_tree(80, 24, &run.session.tree, &mut run.ui, now);
    let Some(key) = run
        .ui
        .shown
        .pulls
        .iter()
        .find(|key| key.request.component == component)
        .cloned()
    else {
        return;
    };
    if run.ui.placed.pulled.get(&key).is_some() {
        return;
    }
    let view = View::Box {
        children: vec![View::text("site")],
        layout: Box::new(bingo_sdk::view::BoxStyle {
            on_key: Some(Action {
                name: "site:ui".into(),
                args: json!({"query":key.request.props["query"]}),
            }),
            ..Default::default()
        }),
    };
    run.ui.placed.pulled.request(&key);
    run.reply(
        Reply::Rendered(run.ui.placed.pulled.epoch(), key, Box::new(view)),
        &mut None,
    );
}

/// The newest key action the site sent and nobody has answered yet.
fn unanswered(session: &TestSession, answered: &[IntentId]) -> Option<(IntentId, Action)> {
    session
        .submissions()
        .into_iter()
        .filter(|(intent, _)| !answered.contains(intent))
        .find_map(|(intent, input)| match input {
            Input::Action { action, .. } => Some((intent, action)),
            _ => None,
        })
}

/// Plays the palette and its kernel: every key is judged after the site has
/// drawn the query before it, and Enter runs the line the query names.
fn play_palette(run: &mut Run, session: &TestSession, now: crate::clock::Now) {
    let mut answered = Vec::new();
    for _ in 0..64 {
        pull(run, Component::Palette, now);
        let Some((intent, action)) = unanswered(session, &answered) else {
            break;
        };
        answered.push(intent.clone());
        let result = if action.args["value"]["key"] == "enter" {
            let handler = crate::palette::request(&run.ui).unwrap().to_owned();
            let line = action.args["query"].as_str().unwrap().to_owned();
            json!({"drawnBy":"palette","surface":{"name":"surface:palette","args":{"handler":handler,"value":{"run":line}}}})
        } else {
            json!({"handled":false})
        };
        run.ack(&intent, &IntentOutcome::Applied { result });
    }
}

fn texts(session: &TestSession) -> Vec<String> {
    session
        .submitted()
        .into_iter()
        .filter_map(|input| match input {
            Input::Text { text, .. } => Some(text),
            _ => None,
        })
        .collect()
}

fn palette_run(now: crate::clock::Now) -> (tempfile::TempDir, Run, Arc<TestSession>) {
    let mut state = state();
    state.config.kernel["render"] = json!({"Palette":[{}],"Sessions":[{}]});
    let session = Arc::new(TestSession::default());
    let mut run = tests::idle_in(state, session.clone(), now.instant);
    // A submitted line goes into the prompt history, kept out of the sources.
    let home = tempfile::tempdir().unwrap();
    run.data_dir = home.path().into();
    (home, run, session)
}

fn type_burst(run: &mut Run, burst: &str) {
    for letter in burst.chars() {
        let pressed = if letter == '\n' {
            key(KeyCode::Enter)
        } else {
            typed(letter)
        };
        run.terminal_event(Term::Key(pressed));
    }
}

#[tokio::test]
async fn a_command_typed_in_one_burst_runs_once_the_palette_answers() {
    let now = still(scene().1);
    let (_home, mut run, session) = palette_run(now);
    type_burst(&mut run, "/ledger\n");
    assert!(
        crate::palette::active(&run.ui),
        "the slash opened the palette"
    );
    assert!(
        session.submitted().is_empty(),
        "nothing judged before a drawing"
    );
    play_palette(&mut run, &session, now);
    assert_eq!(texts(&session), ["/ledger"]);
    assert!(run.typeahead.is_empty());
}

#[tokio::test]
async fn keys_typed_behind_a_command_reach_the_site_it_opens() {
    let now = still(scene().1);
    let (_home, mut run, session) = palette_run(now);
    run.ui.catalogs.commands.push(bingo_sdk::CommandSpec {
        name: "sessions".into(),
        opens: true,
        keys: vec![],
        aliases: vec![],
        hint: String::new(),
        args: bingo_sdk::ArgSpec::None,
        instant: true,
        hidden: false,
        family: "session".into(),
    });
    type_burst(&mut run, "/sessions\nabc");
    play_palette(&mut run, &session, now);
    assert_eq!(
        texts(&session),
        ["/sessions"],
        "the line went to the kernel"
    );
    assert!(!crate::palette::active(&run.ui));
    assert_eq!(run.typeahead.len(), 3, "abc waits for the command's answer");
    assert_eq!(run.ui.composer.text(), "");
    let (intent, _) = session.submissions().last().cloned().unwrap();
    let result = json!({"handled":true,"surface":{"name":"Sessions","args":{}}});
    run.ack(&intent, &IntentOutcome::Applied { result });
    assert!(
        crate::sessions::active(&run.ui),
        "the command opened its site"
    );
    let mut answered: Vec<IntentId> = session
        .submissions()
        .into_iter()
        .map(|(intent, _)| intent)
        .collect();
    for _ in 0..16 {
        // The directory's drawing carries the minute, so it is drawn now.
        let drawn = run.detection.now();
        pull(&mut run, Component::Sessions, drawn);
        let Some((intent, action)) = unanswered(&session, &answered) else {
            break;
        };
        assert_eq!(
            action.args["query"],
            run.ui.composer.text(),
            "each key is judged against the filter it was typed after"
        );
        answered.push(intent.clone());
        let result = json!({"handled":false});
        run.ack(&intent, &IntentOutcome::Applied { result });
    }
    assert_eq!(run.ui.composer.text(), "abc", "the filter holds every key");
    assert!(run.typeahead.is_empty());
}

#[tokio::test]
async fn keys_typed_before_a_dialog_opened_are_never_entered_into_it() {
    let now = still(scene().1);
    for ask in [permission(None, None), question(false, true)] {
        let (_home, mut run, session) = palette_run(now);
        type_burst(&mut run, "/");
        pull(&mut run, Component::Palette, now);
        type_burst(&mut run, "\nya");
        let (enter, _) = unanswered(&session, &[]).unwrap();
        assert_eq!(run.typeahead.len(), 2, "y and a wait behind the enter");
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
        let handler = crate::palette::request(&run.ui).unwrap().to_owned();
        let result = json!({"surface":{"name":"surface:palette","args":{"handler":handler,"value":{"close":true}}}});
        run.ack(&enter, &IntentOutcome::Applied { result });
        assert!(!crate::palette::active(&run.ui));
        assert!(run.session.tree.open_interaction().is_some());
        assert!(!run.ui.dialog.aside, "the ask owns the keyboard now");
        assert!(run.typeahead.is_empty());
        assert_eq!(session.submissions().len(), 1, "no key reached the ask");
        assert!(session.answers().is_empty(), "nothing answered the ask");
        assert_eq!(
            run.ui.notices.first().map(|notice| notice.text.as_str()),
            Some(typeahead::kept_out_notice(2).as_str())
        );
    }
}

#[tokio::test]
async fn keys_wait_no_longer_than_the_command_wait() {
    let now = still(scene().1);
    let (_home, mut run, session) = palette_run(now);
    typeahead::command_sent(&mut run, IntentId::from_raw("slow"));
    type_burst(&mut run, "hi");
    assert_eq!(run.typeahead.len(), 2);
    assert!(
        run.next_draw().is_some(),
        "the loop wakes when the wait ends"
    );
    run.typeahead.expire();
    typeahead::drain(&mut run);
    assert_eq!(run.ui.composer.text(), "hi");
    assert!(run.next_draw().is_none_or(|at| at > Instant::now()));
    assert!(session.submitted().is_empty());
}
