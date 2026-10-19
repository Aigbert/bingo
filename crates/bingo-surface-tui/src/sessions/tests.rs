use super::key;
use super::*;
use crate::test_support::*;
use bingo_sdk::{Contribution, HookContext, Next, Plugin, Registrar};
use std::sync::Arc;
struct Fixture {
    _home: tempfile::TempDir,
    link: Arc<dyn Interceptor>,
    handler: Arc<dyn Command>,
    host: testing::TestHost,
}
impl Fixture {
    fn new(rows: Vec<SessionSummary>) -> Self {
        let home = tempfile::tempdir().unwrap();
        let mut registrar = Registrar::new(
            "sessions",
            json!({}),
            Env::rooted(home.path()),
            Default::default(),
        );
        bingo_mod_sessions::Sessions
            .register(&mut registrar)
            .unwrap();
        let contributions = registrar.into_contributions();
        let link = contributions
            .iter()
            .find_map(|c| match c {
                Contribution::Interceptor { link, .. } if link.id() == "sessions" => {
                    Some(link.clone())
                }
                _ => None,
            })
            .unwrap();
        let handler = contributions
            .iter()
            .find_map(|c| match c {
                Contribution::Command(command) if command.spec().hidden => Some(command.clone()),
                _ => None,
            })
            .unwrap();
        let host = testing::TestHost::with_sessions(rows);
        host.on_fork(|_, _| async { Ok(SessionId::from_raw("fork")) });
        host.on_check_delete(|_| async { Ok(()) });
        host.on_reconfigure(|_, _| async { Ok(()) });
        Self {
            _home: home,
            link,
            handler,
            host,
        }
    }
    async fn paint(&self, ui: &mut Ui, tree: &Tree, now: Now, width: u16, height: u16) -> String {
        draw_tree(width, height, tree, ui, now);
        let key = ui
            .shown
            .pulls
            .iter()
            .find(|k| k.request.component == render::Component::Sessions)
            .unwrap()
            .clone();
        let cx = HookContext {
            session: tree.view().clone(),
            turn: None,
            item: None,
            cwd: tree.viewed().summary.cwd.clone().into(),
            provider: None,
            model: None,
            host: self.host.handle(),
        };
        let value = self
            .link
            .chain(
                catalogue::UI_RENDER,
                &cx,
                serde_json::to_value(&key.request).unwrap(),
                Next::new(|v| Box::pin(async move { Ok(v) })),
            )
            .await
            .unwrap();
        let view: View = serde_json::from_value(value).unwrap();
        assert!(
            view::answer_ok(
                render::Component::Sessions,
                &key.request.props,
                "sessions",
                None,
                &view
            )
            .is_ok()
        );
        ui.placed.pulled.request(&key);
        ui.placed.pulled.answer(key, view);
        draw_tree(width, height, tree, ui, now)
    }
    async fn press(&self, ui: &mut Ui, tree: &Tree, chord: &str, now: Now) -> Vec<Effect> {
        let event = crate::keys::commands::event(chord).unwrap();
        let effects = key(ui, tree, event, now);
        let [Effect::SiteKey { action, .. }] = effects.as_slice() else {
            return effects;
        };
        let cx = CommandContext {
            surface: None,
            session: tree.view().clone(),
            cwd: tree.viewed().summary.cwd.clone().into(),
            host: self.host.handle(),
            cancel: Default::default(),
        };
        match self
            .handler
            .run(&action.args.to_string(), &cx)
            .await
            .unwrap()
        {
            CommandOutcome::Surface { action, .. } => result(ui, &action, now),
            CommandOutcome::Handled { handled: false } => {
                crate::input::palette_edit(ui, tree, event, now)
            }
            _ => vec![],
        }
    }
}
fn summary(id: &str, title: &str) -> SessionSummary {
    let mut summary = state().summary;
    summary.id = SessionId::from_raw(id);
    summary.title = Some(title.into());
    summary.history.messages = Some(38);
    summary.updated_at = ts() - jiff::SignedDuration::from_secs(7200);
    summary
}
fn rows() -> Vec<SessionSummary> {
    let mut root = summary("root", "fix the stale-edit race");
    root.asking = true;
    let mut child = summary("child", "review: action FIFO");
    child.parent = Some(ParentLink {
        session: root.id.clone(),
        item: None,
    });
    child.history.messages = Some(12);
    let mut fixture = summary("fixture", "fixture for checkpoint v2");
    fixture.history.first_ask = Some("can you add a fixture for checkpoint v2?".into());
    let mut scheduled = summary("scheduled", "nightly report");
    scheduled.key = Some("schedule/nightly".into());
    vec![root, child, fixture, scheduled]
}
fn setup() -> (Ui, Tree, Now) {
    let mut state = state();
    state.summary.id = SessionId::from_raw("current");
    state.config.kernel["render"] = json!({"Sessions":[{}]});
    let (mut ui, now) = scene();
    let now = still(now);
    open(&mut ui, now);
    if let Some(Open::Sessions(open)) = ui.focus.open_mut() {
        open.sessions = rows();
    }
    (ui, solo(&state), now)
}
#[tokio::test]
async fn directory_matches_the_sheet_at_both_sizes() {
    let fixture = Fixture::new(rows());
    for (width, height) in [(108, 30), (80, 24)] {
        let (mut ui, tree, now) = setup();
        let screen = fixture.paint(&mut ui, &tree, now, width, height).await;
        insta::assert_snapshot!(format!("sessions_{width}x{height}"), screen);
        assert_eq!(screen.matches('▎').count(), 1);
        assert!(screen.contains("nightly report"));
        assert!(screen.contains("needs you"));
        assert!(screen.contains("review: action FIFO"));
    }
}
#[tokio::test]
async fn letters_filter_and_never_fork_rename_or_delete() {
    for letter in ["f", "r", "d"] {
        let fixture = Fixture::new(rows());
        let (mut ui, tree, now) = setup();
        fixture.paint(&mut ui, &tree, now, 80, 24).await;
        assert!(fixture.press(&mut ui, &tree, letter, now).await.is_empty());
        assert_eq!(ui.composer.text(), letter);
        assert!(!fixture.host.calls().iter().any(|call| matches!(
            call,
            testing::HostCall::Delete { .. }
                | testing::HostCall::Reconfigure { .. }
                | testing::HostCall::Open { .. }
        )));
    }
}
#[tokio::test]
async fn typing_filters_and_enter_opens_the_selected_identity() {
    let fixture = Fixture::new(rows());
    let (mut ui, tree, now) = setup();
    for chord in ["f", "i", "x", "t", "u", "r", "e"] {
        fixture.paint(&mut ui, &tree, now, 80, 24).await;
        fixture.press(&mut ui, &tree, chord, now).await;
    }
    let screen = fixture.paint(&mut ui, &tree, now, 80, 24).await;
    assert!(screen.contains("1 of 4"));
    assert!(!screen.contains("nightly report"));
    assert_eq!(
        fixture.press(&mut ui, &tree, "enter", now).await,
        vec![Effect::Open(SessionSelector::ById {
            id: SessionId::from_raw("fixture")
        })]
    );
}
#[tokio::test]
async fn refresh_keeps_selection_by_id_and_escape_restores_the_draft() {
    let fixture = Fixture::new(rows());
    let (mut ui, tree, now) = setup();
    if let Some(Open::Sessions(open)) = ui.focus.open_mut() {
        open.draft = "keep this".into();
    }
    fixture.paint(&mut ui, &tree, now, 80, 24).await;
    fixture.press(&mut ui, &tree, "down", now).await;
    if let Some(Open::Sessions(open)) = ui.focus.open_mut() {
        open.sessions.reverse();
    }
    let screen = fixture.paint(&mut ui, &tree, now, 80, 24).await;
    assert!(
        screen
            .lines()
            .any(|line| line.contains('▎') && line.contains("review: action FIFO"))
    );
    fixture.press(&mut ui, &tree, "esc", now).await;
    assert_eq!(ui.composer.text(), "keep this");
}
#[tokio::test]
async fn a_long_title_cannot_hide_attention_at_eighty_columns() {
    let fixture = Fixture::new(rows());
    let (mut ui, tree, now) = setup();
    if let Some(Open::Sessions(open)) = ui.focus.open_mut() {
        open.sessions[0].title = Some("long title ".repeat(30));
    }
    let screen = fixture.paint(&mut ui, &tree, now, 80, 24).await;
    let selected = screen.lines().find(|line| line.contains('▎')).unwrap();
    assert!(selected.contains("needs you"), "{selected}");
}
