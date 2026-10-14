use super::*;
use crate::{
    doubles::{Recorder, TestSession},
    test_support::*,
};
use bingo_sdk::{Contribution, Plugin, Registrar, View};
use serde_json::json;
use std::sync::Arc;

fn contributions() -> Vec<Contribution> {
    let mut registrar = Registrar::new(
        "hello",
        json!({}),
        bingo_sdk::Env::rooted("/unused"),
        Default::default(),
    );
    bingo_mod_hello::Hello.register(&mut registrar).unwrap();
    registrar.into_contributions()
}
fn specs() -> Vec<CommandSpec> {
    contributions()
        .into_iter()
        .filter_map(|contribution| match contribution {
            Contribution::Command(command) if !command.spec().hidden => Some(command.spec()),
            _ => None,
        })
        .collect()
}
async fn drawing() -> View {
    let command = contributions()
        .into_iter()
        .find_map(|contribution| match contribution {
            Contribution::Command(command) if !command.spec().hidden => Some(command),
            _ => None,
        })
        .unwrap();
    let outcome = command
        .run(
            "",
            &bingo_sdk::CommandContext {
                session: state().summary.id,
                cwd: "/unused".into(),
                host: bingo_sdk::testing::NoHost::handle(),
                cancel: Default::default(),
            },
        )
        .await
        .unwrap();
    let bingo_sdk::CommandOutcome::View { view } = outcome else {
        panic!("view")
    };
    view
}
#[tokio::test]
async fn default_rebound_and_static_keys_use_the_command_catalogue() {
    for (settings, key, sends) in [
        (json!({}), ctrl('x'), true),
        (
            json!({"alt+h":"/hello"}),
            crossterm::event::KeyEvent::new(
                crossterm::event::KeyCode::Char('h'),
                crossterm::event::KeyModifiers::ALT,
            ),
            true,
        ),
        (json!({"alt+h":"/hello"}), ctrl('x'), false),
        (json!({"ctrl+c":"/hello"}), ctrl('c'), false),
    ] {
        let home = tempfile::tempdir().unwrap();
        let mut state = state();
        state.config.kernel["keys"] = settings.clone();
        let session = Arc::new(TestSession::default());
        let now = scene().1;
        let mut run = tests::idle_in(state, session.clone(), now.instant);
        run.data_dir = home.path().into();
        run.ui.catalogs.commands = specs();
        run.ui.composer.set("draft");
        site_keys::notices(&mut run);
        let mut screen = Recorder::default();
        run.paint(&mut screen, Wake::Frame, now).unwrap();
        if settings.get("ctrl+c").is_some() {
            assert!(
                screen.last().contains("belongs to the surface"),
                "{}",
                screen.last()
            );
        }
        run.terminal_event(Term::Key(key));
        assert_eq!(session.submitted().len(), usize::from(sends));
        if sends {
            assert!(matches!(&session.submitted()[0], Input::Text {text,..} if text == "/hello"));
            assert_eq!(run.ui.composer.text(), "draft");
        }
    }
}
fn pane(view: View) -> SessionState {
    let mut state = state();
    let slot: bingo_sdk::slot::Slot = serde_json::from_value(json!({"kind":"pane","id":"keys","title":"Keys","focus":true,"asked":true,"closeOnEscape":true,"holdToasts":false,"rows":8,"view":view})).unwrap();
    state
        .signals
        .entry("hello".into())
        .or_default()
        .insert("pane".into(), bingo_sdk::slot::with(json!({}), [slot]));
    state
}
#[tokio::test]
async fn focused_root_handler_gets_keys_and_unhandled_returns_to_controls() {
    let state = pane(drawing().await);
    let session = Arc::new(TestSession::default());
    let now = scene().1;
    let mut run = tests::idle_in(state, session.clone(), now.instant);
    let mut screen = Recorder::default();
    run.paint(&mut screen, Wake::Frame, now).unwrap();
    assert!(screen.last().contains("Hello from a built-in mod"));
    run.terminal_event(Term::Key(typed('x')));
    assert!(
        matches!(&session.submitted()[0], Input::Action { action } if action.args["value"] == json!({"key":"x"}))
    );
    let intent = run.site_keys.keys().next().unwrap().clone();
    run.ack(
        &intent,
        &IntentOutcome::Applied {
            result: json!({"handled":false}),
        },
    );
    assert_eq!(run.ui.composer.text(), "x");
    for key in [key(crossterm::event::KeyCode::Esc), ctrl('c')] {
        run.ui.focus = crate::focus::Focus::Pane(format!(
            "{}/hello/keys",
            run.session.tree.viewed().summary.id
        ));
        run.terminal_event(Term::Key(key));
    }
    assert!(!session.submitted().iter().any(|input| matches!(input, Input::Action {action} if action.args["value"]["key"] == "esc" || action.args["value"]["key"] == "ctrl+c")));
}
#[tokio::test]
async fn an_immediately_ready_tree_is_drawn_in_the_requesting_pass() {
    let mut state = state();
    state.config.kernel["render"] = json!({"Welcome":[{}]});
    let host = bingo_sdk::testing::TestHost::default();
    let view = drawing().await;
    host.on_render(move |_, _| {
        let view = view.clone();
        async move { Ok(view) }
    });
    let now = scene().1;
    let mut run = tests::idle(now.instant);
    run.session.tree = solo(&state);
    run.host = host.handle();
    let mut screen = Recorder::default();
    run.paint(&mut screen, Wake::Frame, now).unwrap();
    assert_eq!(screen.frames.len(), 1);
    assert!(
        screen.last().contains("Hello from a built-in mod"),
        "{}",
        screen.last()
    );
}

#[tokio::test]
async fn a_malformed_handler_cannot_panic_the_surface() {
    let mut view = drawing().await;
    if let View::Box { layout, .. } = &mut view {
        layout.on_key.as_mut().unwrap().args = json!("invalid");
    }
    let state = pane(view);
    let (mut ui, now) = scene();
    draw_sized(120, 40, &state, &mut ui, now);
    assert!(crate::pane::key_handler(&ui, &state, typed('x')).is_none());
}
