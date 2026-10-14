use super::*;
use crate::test_support::key;
use crate::test_support::*;
use bingo_sdk::{Contribution, HookContext, Next, Plugin, Registrar};
use crossterm::event::KeyModifiers;
use serde_json::Value;
use std::sync::Arc;

struct Fixture {
    _home: tempfile::TempDir,
    link: Arc<dyn Interceptor>,
    handler: Arc<dyn Command>,
}
impl Fixture {
    fn new() -> Self {
        let home = tempfile::tempdir().unwrap();
        let mut registrar = Registrar::new(
            "palette",
            json!({}),
            Env::rooted(home.path()),
            Default::default(),
        );
        bingo_mod_palette::Palette.register(&mut registrar).unwrap();
        let contributions = registrar.into_contributions();
        Self {
            _home: home,
            link: contributions
                .iter()
                .find_map(|c| {
                    if let Contribution::Interceptor { link, .. } = c {
                        Some(link.clone())
                    } else {
                        None
                    }
                })
                .unwrap(),
            handler: contributions
                .iter()
                .find_map(|c| {
                    if let Contribution::Command(command) = c
                        && command.spec().hidden
                    {
                        Some(command.clone())
                    } else {
                        None
                    }
                })
                .unwrap(),
        }
    }
    async fn paint(
        &self,
        ui: &mut Ui,
        state: &SessionState,
        now: Now,
        width: u16,
        height: u16,
    ) -> String {
        draw_sized(width, height, state, ui, now);
        let key = ui
            .shown
            .pulls
            .iter()
            .find(|key| key.request.component == render::Component::Palette)
            .unwrap()
            .clone();
        let host = testing::TestHost::default();
        host.on_service_any(|key| {
            if key != command::COMMAND_CATALOG.name() {
                return None;
            }
            let reader: Arc<dyn command::CommandCatalog> = Arc::new(Values);
            Some(Arc::new(reader))
        });
        let cx = HookContext {
            session: state.summary.id.clone(),
            turn: None,
            item: None,
            cwd: state.summary.cwd.clone().into(),
            provider: None,
            model: None,
            host: host.handle(),
        };
        let value = self
            .link
            .chain(
                catalogue::UI_RENDER,
                &cx,
                serde_json::to_value(&key.request).unwrap(),
                Next::new(|value| Box::pin(async move { Ok(value) })),
            )
            .await
            .unwrap();
        let view: View = serde_json::from_value(value).unwrap();
        assert!(
            view::answer_ok(
                render::Component::Palette,
                &key.request.props,
                "palette",
                None,
                &view
            )
            .is_ok()
        );
        ui.placed.pulled.request(&key);
        ui.placed.pulled.answer(key, view);
        draw_sized(width, height, state, ui, now)
    }
    async fn press(
        &self,
        ui: &mut Ui,
        state: &SessionState,
        key: KeyEvent,
        now: Now,
    ) -> Vec<Effect> {
        let tree = solo(state);
        let effects = crate::input::on_key(ui, &tree, key, now);
        let Some(Effect::SiteKey { action, .. }) = effects.first() else {
            return effects;
        };
        let cx = CommandContext {
            session: state.summary.id.clone(),
            cwd: state.summary.cwd.clone().into(),
            host: testing::NoHost::handle(),
            cancel: Default::default(),
        };
        match self
            .handler
            .run(&action.args.to_string(), &cx)
            .await
            .unwrap()
        {
            CommandOutcome::Surface { action, .. } => result(ui, &tree, &action, now),
            CommandOutcome::Handled { handled: false } => {
                crate::input::palette_edit(ui, &tree, key, now)
            }
            _ => vec![],
        }
    }
}
fn setup() -> (SessionState, Ui, Now) {
    let mut state = state();
    state.config.kernel["render"] = json!({"Palette":[{}]});
    state
        .signals
        .entry("permissions".into())
        .or_default()
        .insert(
            "mode".into(),
            slot::with(
                json!({}),
                [slot::Slot::Mode {
                    current: "default".into(),
                    modes: vec![ActionItem {
                        label: "default".into(),
                        action: Action {
                            name: "permission".into(),
                            args: json!("default"),
                        },
                        key: None,
                    }],
                }],
            ),
        );
    let (mut ui, now) = scene();
    ui.catalogs.commands=[("model",json!({"kind":"catalog","source":"models"})),("think",json!({"kind":"words","values":["off","low","high"]})),("rename",json!({"kind":"free","hint":"a name for this session"})),("permission",json!({"kind":"words","values":["default","acceptEdits","plan"]})),("resume",json!({"kind":"none"}))].into_iter().map(|(name,args)|serde_json::from_value(json!({"name":name,"hint":format!("choose {name}"),"args":args,"family":"session","keys":if name=="resume"{vec!["ctrl+r"]}else{vec![]}})).unwrap()).collect();
    ui.catalogs.values.insert(
        "models".into(),
        ["fake/small", "fake/large"]
            .map(|id| CatalogEntry {
                id: id.into(),
                label: id.into(),
                meta: Value::Null,
            })
            .to_vec(),
    );
    (state, ui, still(now))
}
fn sessions(ui: &mut Ui, state: &SessionState) {
    let Some(Open::Palette(open)) = ui.focus.open_mut() else {
        panic!("palette");
    };
    let mut other = state.summary.clone();
    other.id = SessionId::from_raw("ses_other");
    other.title = Some("model migration".into());
    other.messages = Some(12);
    open.sessions = vec![state.summary.clone(), other];
}
#[tokio::test]
async fn prototype_scenarios_at_both_sizes() {
    for (width, height) in [(108, 30), (80, 24)] {
        for (name, first, query) in [
            ("commands", PaletteOpen::Commands, "/"),
            ("keys", PaletteOpen::Keys, ""),
            ("sessions", PaletteOpen::Sessions, ""),
            ("mo", PaletteOpen::Commands, "/mo"),
            ("think", PaletteOpen::Commands, "/think "),
            ("rename", PaletteOpen::Commands, "/rename release notes"),
            ("permission", PaletteOpen::Commands, "/permission "),
        ] {
            let fixture = Fixture::new();
            let (state, mut ui, now) = setup();
            open(&mut ui, first, now);
            sessions(&mut ui, &state);
            ui.composer.set(query);
            let screen = fixture.paint(&mut ui, &state, now, width, height).await;
            insta::assert_snapshot!(format!("palette_{name}_{width}x{height}"), screen);
        }
    }
}
#[tokio::test]
async fn search_arguments_escape_and_resume_use_the_same_site() {
    let fixture = Fixture::new();
    let (state, mut ui, now) = setup();
    assert!(matches!(
        crate::input::on_key(&mut ui, &solo(&state), typed('/'), now).as_slice(),
        [Effect::ListStored]
    ));
    sessions(&mut ui, &state);
    ui.composer.set("/mo");
    let screen = fixture.paint(&mut ui, &state, now, 80, 24).await;
    assert!(
        screen.contains("model migration")
            && screen.contains("commands")
            && screen.contains("sessions"),
        "{screen}"
    );
    fixture
        .press(&mut ui, &state, key(KeyCode::Enter), now)
        .await;
    assert_eq!(ui.composer.text(), "/model ");
    fixture.paint(&mut ui, &state, now, 80, 24).await;
    fixture.press(&mut ui, &state, key(KeyCode::Esc), now).await;
    assert_eq!(ui.composer.text(), "/");
    fixture.paint(&mut ui, &state, now, 80, 24).await;
    fixture.press(&mut ui, &state, key(KeyCode::Esc), now).await;
    assert!(!active(&ui));
    open(&mut ui, PaletteOpen::Sessions, now);
    sessions(&mut ui, &state);
    fixture.paint(&mut ui, &state, now, 80, 24).await;
    fixture
        .press(&mut ui, &state, key(KeyCode::Down), now)
        .await;
    fixture.paint(&mut ui, &state, now, 80, 24).await;
    let effects = fixture
        .press(&mut ui, &state, key(KeyCode::Enter), now)
        .await;
    assert!(
        matches!(&effects[0],Effect::Open(SessionSelector::ById{id}) if id.as_str()=="ses_other")
    );
}
#[test]
fn help_resume_and_shift_tab_keep_their_own_routes() {
    let (state, mut ui, now) = setup();
    let tree = solo(&state);
    for (key, line) in [(typed('?'), "/help"), (ctrl('r'), "/resume")] {
        let effects = crate::input::on_key(&mut ui, &tree, key, now);
        assert!(matches!(&effects[0],Effect::Submit(Input::Text{text,..}) if text==line));
    }
    crate::input::on_key(
        &mut ui,
        &tree,
        KeyEvent::new(KeyCode::BackTab, KeyModifiers::SHIFT),
        now,
    );
    assert!(!active(&ui));
}
#[test]
fn disabled_palette_lists_names_and_runs_typed_commands() {
    let (mut state, mut ui, now) = setup();
    state.config.kernel["render"] = json!({});
    let tree = solo(&state);
    crate::input::on_key(&mut ui, &tree, typed('/'), now);
    let screen = draw_sized(80, 24, &state, &mut ui, now);
    assert!(screen.contains("/model"));
    assert!(!screen.contains("choose model"));
    ui.composer.set("/rename explicit");
    let effects = crate::input::on_key(&mut ui, &tree, key(KeyCode::Enter), now);
    assert!(matches!(&effects[0],Effect::Submit(Input::Text{text,..}) if text=="/rename explicit"));
}
#[tokio::test]
async fn stale_query_and_engine_answers_cannot_trap_the_keyboard() {
    let fixture = Fixture::new();
    let (state, mut ui, now) = setup();
    open(&mut ui, PaletteOpen::Commands, now);
    fixture.paint(&mut ui, &state, now, 80, 24).await;
    ui.composer.set("/rename ");
    assert!(waiting(&ui, &state));
    fixture.paint(&mut ui, &state, now, 80, 24).await;
    assert!(!waiting(&ui, &state));
    let key = ui
        .shown
        .pulls
        .iter()
        .find(|key| key.request.component == render::Component::Palette)
        .unwrap()
        .clone();
    ui.placed.pulled.answer(key.clone(), key.request.engine());
    assert!(!waiting(&ui, &state));
}

struct Values;
impl command::CommandCatalog for Values {
    fn catalog(&self, mut catalog: Catalog, view: &command::SessionView<'_>) -> Catalog {
        assert!(slot::placed(view.state).iter().any(
            |placed| matches!(&placed.slot, slot::Slot::Mode {current, ..} if current=="default")
        ));
        for entry in &mut catalog.entries {
            entry.meta["now"] = json!(match entry.id.as_str() {
                "model" => "fake/small",
                "think" => "high",
                "rename" => "Release notes",
                "permission" => "default",
                _ => "",
            });
        }
        catalog
    }
}

#[tokio::test]
async fn tab_does_not_run_arguments_and_escape_leaves_free_text_at_once() {
    let fixture = Fixture::new();
    let (state, mut ui, now) = setup();
    open(&mut ui, PaletteOpen::Commands, now);
    ui.composer.set("/think ");
    fixture.paint(&mut ui, &state, now, 80, 24).await;
    assert!(
        fixture
            .press(&mut ui, &state, key(KeyCode::Tab), now)
            .await
            .is_empty()
    );
    assert_eq!(ui.composer.text(), "/think ");
    let effects = fixture
        .press(&mut ui, &state, key(KeyCode::Enter), now)
        .await;
    assert!(matches!(&effects[0], Effect::Submit(Input::Text {text,..}) if text=="/think high"));
    open(&mut ui, PaletteOpen::Commands, now);
    ui.composer.set("/rename several words");
    fixture.paint(&mut ui, &state, now, 80, 24).await;
    fixture.press(&mut ui, &state, key(KeyCode::Esc), now).await;
    assert_eq!(ui.composer.text(), "/");
}

#[tokio::test]
async fn choosing_the_mode_key_replays_shift_tab_after_closing() {
    let fixture = Fixture::new();
    let (state, mut ui, now) = setup();
    open(&mut ui, PaletteOpen::Keys, now);
    fixture.paint(&mut ui, &state, now, 80, 24).await;
    let effects = fixture
        .press(&mut ui, &state, key(KeyCode::Enter), now)
        .await;
    assert!(!active(&ui));
    assert!(
        matches!(&effects[0], Effect::Submit(Input::Action {action}) if action.name == "permission")
    );
}
