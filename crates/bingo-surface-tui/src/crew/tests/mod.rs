use crate::test_support::*;
use bingo_mod_crew::{
    draw,
    state::{Panel, Rows, Tab},
};
use bingo_sdk::*;
use bingo_services::{
    agents_control::AgentRow,
    rooms_control::RoomRow,
    tasks_write::{Status, TaskRow},
};
use crossterm::event::KeyCode;
use serde_json::json;
fn rows() -> Rows {
    let agents = [
        ("reviewer", "working", "sonnet"),
        ("fixer", "idle", "opus"),
        ("scout", "stopped", "haiku"),
    ]
    .map(|(name, word, model)| {
        let mut summary = state().summary;
        summary.title = Some(name.into());
        summary.model = Some(model.into());
        summary.history.first_ask = Some("review the fork cut for off-by-one turns".into());
        summary.usage.input_tokens = 12000;
        summary.busy = word == "working";
        AgentRow { summary }
    })
    .to_vec();
    Rows {
        agents: Some(agents),
        rooms: Some(vec![RoomRow {
            id: SessionId::from_raw("ses_room"),
            name: "#design".into(),
            purpose: Some("Review the fork boundary".into()),
            seats: vec!["reviewer".into(), "fixer".into()],
            holder: "parent".into(),
            closed: false,
        }]),
        tasks: Some(vec![TaskRow {
            id: 1,
            subject: "Review the boundary".into(),
            status: Status::Pending,
            owner: Some("reviewer".into()),
        }]),
    }
}
fn placed(state: &mut SessionState, view: View) {
    let mut size = slot::PaneSize::from_rows(18);
    size.rows.max = slot::Length::Percent(80);
    size.columns.ideal = slot::Length::Percent(100);
    let pane = slot::Slot::Pane {
        id: "crew".into(),
        title: "crew".into(),
        focus: false,
        asked: true,
        close_on_escape: false,
        hold_toasts: false,
        place: slot::above(),
        size,
        view: slot::Shown::View(view),
    };
    state
        .signals
        .entry("bingo.mod.crew".into())
        .or_default()
        .insert("crew".into(), slot::with(json!({}), [pane]).into());
}
#[test]
fn crew_scenes_at_both_terminal_sizes() {
    for (width, height) in [(108, 30), (80, 24)] {
        for (scene, tab) in [
            ("agents", Tab::Agents),
            ("rooms", Tab::Rooms),
            ("tasks", Tab::Tasks),
            ("empty", Tab::Agents),
        ] {
            let rows = if scene == "empty" {
                Rows {
                    agents: Some(vec![]),
                    rooms: Some(vec![]),
                    tasks: Some(vec![]),
                }
            } else {
                rows()
            };
            let panel = Panel {
                tab,
                ..Default::default()
            };
            let mut state = state();
            placed(&mut state, draw::tree(&panel, &rows, &[], 18));
            let (mut ui, now) = scene_state();
            ui.focus =
                crate::focus::Focus::Pane(format!("{}/bingo.mod.crew/crew", state.summary.id));
            let screen = draw_tree(width, height, &solo(&state), &mut ui, now);
            insta::assert_snapshot!(format!("crew_{scene}_{width}x{height}"), screen);
        }
    }
}
fn scene_state() -> (crate::ui::Ui, crate::clock::Now) {
    scene()
}
#[test]
fn ctrl_n_is_bound_only_when_the_command_is_installed() {
    let (mut ui, now) = scene();
    let state = state();
    let mut registrar = Registrar::new(
        "bingo.mod.crew",
        json!({}),
        Env::rooted("/tmp"),
        Default::default(),
    );
    bingo_mod_crew::Crew::default()
        .register(&mut registrar)
        .unwrap();
    ui.catalogs.commands = registrar
        .into_contributions()
        .into_iter()
        .filter_map(|c| {
            if let Contribution::Command(c) = c {
                Some(c.spec())
            } else {
                None
            }
        })
        .collect();
    assert!(
        ui.catalogs
            .commands
            .iter()
            .all(|spec| spec.foreign_handler("bingo.mod.crew").is_none())
    );
    assert!(
        ui.catalogs
            .commands
            .iter()
            .any(|spec| spec.name == CommandSpec::ui_name("bingo.mod.crew"))
    );
    let key = crossterm::event::KeyEvent::new(
        crossterm::event::KeyCode::Char('n'),
        crossterm::event::KeyModifiers::CONTROL,
    );
    assert_eq!(
        crate::keys::commands::line(&ui, &state, key).as_deref(),
        Some("/crew")
    );
    let effects = crate::input::on_key(&mut ui, &solo(&state), key, now);
    assert!(!effects.is_empty());
    ui.catalogs.commands.clear();
    assert!(crate::keys::commands::line(&ui, &state, key).is_none());
}
#[test]
fn absent_rooms_have_no_tab() {
    let mut rows = rows();
    rows.rooms = None;
    let view = draw::tree(&Panel::default(), &rows, &[], 18).fold();
    assert!(!view.contains("rooms ("));
}

mod live;

#[test]
fn a_strip_click_without_the_mod_opens_conversations() {
    let (mut ui, now) = scene();
    let state = state();
    let effects = crate::crew::open(&mut ui, &solo(&state), now);
    assert!(matches!(
        effects.as_slice(),
        [crate::effect::Effect::ListStored]
    ));
    assert!(matches!(ui.focus.open(), crate::ui::Open::Palette(open)
        if open.first == component::PaletteOpen::Sessions));
}

#[test]
fn crew_tab_switch_reaches_the_mod_without_taking_focus_keys() {
    let mut state = state();
    placed(&mut state, draw::tree(&Panel::default(), &rows(), &[], 18));
    let tree = solo(&state);
    let (mut ui, now) = scene();
    ui.focus = crate::focus::Focus::Pane(format!("{}/bingo.mod.crew/crew", state.summary.id));
    draw_tree(108, 30, &tree, &mut ui, now);
    let effects = crate::input::on_key(&mut ui, &tree, key(KeyCode::Char(']')), now);
    assert!(
        matches!(&effects[..], [crate::effect::Effect::SiteKey { action, .. }]
        if action.name == CommandSpec::ui_name("bingo.mod.crew")
            && action.args["value"]["key"] == "]")
    );
    let effects = crate::input::on_key(&mut ui, &tree, key(KeyCode::Tab), now);
    assert!(effects.is_empty());
    assert!(matches!(ui.focus, crate::focus::Focus::Composer));
}

#[test]
fn escape_is_delivered_to_the_confirmation_instead_of_closing_the_pane() {
    for confirm in [
        bingo_mod_crew::state::Confirmation::Stop("scout".into()),
        bingo_mod_crew::state::Confirmation::Dismiss("scout".into()),
        bingo_mod_crew::state::Confirmation::CloseRoom("design".into()),
    ] {
        let mut state = state();
        let panel = Panel {
            confirm: Some(confirm),
            ..Default::default()
        };
        placed(&mut state, draw::tree(&panel, &rows(), &[], 18));
        let tree = solo(&state);
        let (mut ui, now) = scene();
        ui.focus = crate::focus::Focus::Pane(format!("{}/bingo.mod.crew/crew", state.summary.id));
        draw_tree(80, 24, &tree, &mut ui, now);
        let effects = crate::input::on_key(&mut ui, &tree, key(KeyCode::Esc), now);
        assert!(
            matches!(&effects[..], [crate::effect::Effect::SiteKey { action, .. }]
            if action.args["value"]["key"] == "esc" && action.args["event"] == "press")
        );
        assert!(matches!(ui.focus, crate::focus::Focus::Pane(_)));
    }
}
