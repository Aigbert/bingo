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
async fn directory_matches_the_sketch_at_both_sizes() {
    let fixture = Fixture::new(rows());
    for (width, height) in [(108, 30), (80, 24)] {
        let (mut ui, tree, now) = setup();
        let screen = fixture.paint(&mut ui, &tree, now, width, height).await;
        insta::assert_snapshot!(format!("sessions_{width}x{height}"), screen);
        assert_eq!(screen.matches('❯').count(), 1);
        assert!(screen.contains("schedule (1)"));
        assert!(screen.contains("review: action FIFO"));
    }
}
#[tokio::test]
async fn scope_paging_filter_fork_and_rename_are_client_actions() {
    let fixture = Fixture::new(rows());
    let (mut ui, tree, now) = setup();
    fixture.paint(&mut ui, &tree, now, 108, 30).await;
    let effects = fixture.press(&mut ui, &tree, "f", now).await;
    assert!(
        matches!(&effects[..], [Effect::Open(SessionSelector::ById { id })] if id.as_str() == "fork")
    );
    let (mut ui, tree, now) = setup();
    fixture.paint(&mut ui, &tree, now, 108, 30).await;
    let effects = fixture.press(&mut ui, &tree, "r", now).await;
    assert!(effects.is_empty());
    assert!(matches!(ui.focus.open(), Open::Sessions(_)));
    assert!(
        fixture
            .paint(&mut ui, &tree, now, 108, 30)
            .await
            .contains("save title")
    );
    fixture.press(&mut ui, &tree, "ctrl+u", now).await;
    fixture.press(&mut ui, &tree, "x", now).await;
    assert_eq!(
        fixture.press(&mut ui, &tree, "enter", now).await,
        vec![Effect::ListSessions]
    );
    assert!(fixture.host.calls().iter().any(|call| matches!(call,
        testing::HostCall::Reconfigure { session, change: bingo_sdk::SessionChange::Title(title) }
        if session.as_str() == "root" && title == "x")));
    assert!(
        !fixture
            .host
            .calls()
            .iter()
            .any(|call| matches!(call, testing::HostCall::Open { .. }))
    );
    let (mut ui, tree, now) = setup();
    fixture.paint(&mut ui, &tree, now, 108, 30).await;
    assert_eq!(
        fixture.press(&mut ui, &tree, "tab", now).await,
        vec![Effect::ListSessions]
    );
    assert!(matches!(ui.focus.open(), Open::Sessions(o) if o.scope == SessionsScope::All));
}
#[tokio::test]
async fn delete_names_children_then_removes_them_first() {
    let fixture = Fixture::new(rows());
    let (mut ui, tree, now) = setup();
    fixture.paint(&mut ui, &tree, now, 80, 24).await;
    fixture.press(&mut ui, &tree, "d", now).await;
    let screen = fixture.paint(&mut ui, &tree, now, 80, 24).await;
    assert!(screen.contains("Delete 2 session(s), children first?"));
    assert!(screen.contains("review: action FIFO"));
    fixture.press(&mut ui, &tree, "enter", now).await;
    let deleted = fixture
        .host
        .calls()
        .into_iter()
        .filter_map(|call| match call {
            testing::HostCall::Delete { session } => Some(session.to_string()),
            _ => None,
        })
        .collect::<Vec<_>>();
    assert_eq!(deleted, ["root"]);
}

#[tokio::test]
async fn more_filter_and_group_expansion_stay_in_the_directory() {
    let fixture = Fixture::new(rows());
    let (mut ui, tree, now) = setup();
    if let Some(Open::Sessions(open)) = ui.focus.open_mut() {
        open.limit = 1;
    }
    fixture.paint(&mut ui, &tree, now, 80, 24).await;
    fixture.press(&mut ui, &tree, "down", now).await;
    fixture.paint(&mut ui, &tree, now, 80, 24).await;
    assert_eq!(
        fixture.press(&mut ui, &tree, "enter", now).await,
        vec![Effect::ListSessions]
    );
    assert!(matches!(ui.focus.open(), Open::Sessions(o) if o.limit == 51));
    fixture.paint(&mut ui, &tree, now, 80, 24).await;
    fixture.press(&mut ui, &tree, "/", now).await;
    for chord in ["f", "i", "x"] {
        fixture.paint(&mut ui, &tree, now, 80, 24).await;
        fixture.press(&mut ui, &tree, chord, now).await;
    }
    let screen = fixture.paint(&mut ui, &tree, now, 80, 24).await;
    assert_eq!(ui.composer.text(), "fix");
    assert!(!screen.contains("schedule (1)"));
    fixture.press(&mut ui, &tree, "esc", now).await;
    fixture.paint(&mut ui, &tree, now, 80, 24).await;
    for _ in 0..3 {
        fixture.press(&mut ui, &tree, "down", now).await;
        fixture.paint(&mut ui, &tree, now, 80, 24).await;
    }
    fixture.press(&mut ui, &tree, "enter", now).await;
    assert!(
        fixture
            .paint(&mut ui, &tree, now, 80, 24)
            .await
            .contains("nightly report")
    );
}
#[test]
fn disabled_directory_leaves_the_recent_palette_available() {
    let (mut ui, now) = scene();
    let effects = crate::palette::command(&mut ui, "/sessions", now).unwrap();
    assert!(effects.is_empty());
    assert!(!active(&ui));
    assert!(crate::palette::command(&mut ui, "/resume", now).is_some());
    assert!(crate::palette::active(&ui));
}

#[tokio::test]
async fn a_long_title_cannot_hide_age_count_or_attention_at_eighty_columns() {
    let fixture = Fixture::new(rows());
    let (mut ui, tree, now) = setup();
    if let Some(Open::Sessions(open)) = ui.focus.open_mut() {
        open.sessions[0].title = Some("long title ".repeat(30));
    }
    let screen = fixture.paint(&mut ui, &tree, now, 80, 24).await;
    let selected = screen.lines().find(|line| line.contains('❯')).unwrap();
    assert!(selected.contains("38 msg"), "{selected}");
    assert!(selected.contains("● asking"), "{selected}");
    assert!(selected.contains("ago"), "{selected}");
}
