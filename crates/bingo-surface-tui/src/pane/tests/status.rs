use crate::test_support::*;
use bingo_mod_status::{draw, rows};
use bingo_sdk::{
    slot::{Length, PaneSize, Shown, Slot},
    *,
};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use serde_json::json;

fn scenario(which: &str) -> (SessionState, Vec<CatalogEntry>) {
    let mut state = state();
    state.summary.title = Some("The release".into());
    state.context = Some(serde_json::from_value(json!({"used":42000,"window":200000,"trigger":180000,"parts":[{"name":"system","tokens":3000},{"name":"tools","tokens":6000},{"name":"chat","tokens":33000},{"name":"pictures","tokens":0}]})).unwrap());
    state.config.plugins.insert(
        "bingo.permissions".into(),
        json!({"mode":"default","rules":[]}),
    );
    if which == "long" {
        let context = state.context.as_mut().unwrap();
        context.used = 168000;
        let parts = context.parts.as_mut().unwrap();
        parts[2].tokens = 155000;
        parts[3].tokens = 4000;
        state.summary.usage = Usage {
            input_tokens: 86800,
            cache_read_tokens: 1153200,
            output_tokens: 88000,
            ..Default::default()
        };
        state.config.plugins.insert(
            "bingo.permissions".into(),
            json!({"mode":"acceptEdits","rules":["Edit(src/**)","Bash(cargo:*)"]}),
        );
    }
    let plugins = vec![CatalogEntry {
        id: "review".into(),
        label: "review".into(),
        meta: json!({"state": match which {"failed" => PluginState::Failed { why: "failed".into() }, "waiting" => PluginState::Waiting { changed: false }, _ => PluginState::On}}),
    }];
    (state, plugins)
}
fn put(state: &mut SessionState, view: View, height: u16) {
    let mut size = PaneSize::from_rows(height / 2);
    size.rows.max = Length::Percent(50);
    size.columns.ideal = Length::Percent(100);
    let pane = Slot::Pane {
        id: "status".into(),
        title: String::new(),
        focus: true,
        close_on_escape: true,
        hold_toasts: true,
        asked: true,
        place: slot::above(),
        size,
        view: Shown::View(view),
    };
    state
        .signals
        .entry("bingo.mod.status".into())
        .or_default()
        .insert("status".into(), slot::with(json!({}), [pane]).into());
}
#[test]
fn status_scenarios_match_at_both_terminal_sizes() {
    for (width, height) in [(108, 30), (80, 24)] {
        for which in ["overview", "long", "failed", "waiting", "signin"] {
            let (mut state, plugins) = scenario(which);
            let status = if which == "signin" {
                json!({"kind": "needsAuth", "why": "signed out"})
            } else {
                json!({"kind": "connected", "tools": 8})
            };
            let servers: Vec<_> = serde_json::from_value(json!([{
                "name": "github",
                "transport": {"type": "http", "url": "https://mcp.example.com"},
                "status": status
            }]))
            .unwrap();
            let price = (which == "long").then_some(bingo_sdk::spent::Price {
                input: Some(3.0),
                output: Some(15.0),
                cache_read: Some(0.3),
                cache_write: None,
            });
            let rows = rows::rows(&(&state).into(), &plugins, &servers, price);
            let selected = match which {
                "long" => "context",
                "failed" | "waiting" => "plugins",
                "signin" => "mcp",
                _ => "session",
            };
            put(
                &mut state,
                draw::tree(&rows, selected, width, (height - 4) / 2, 2520),
                height,
            );
            let (mut ui, now) = scene();
            ui.focus =
                crate::focus::Focus::Pane(format!("{}/bingo.mod.status/status", state.summary.id));
            let screen = draw_tree(width, height, &solo(&state), &mut ui, now);
            assert!(ui.shown.seating.panes[0].area.height <= height / 2);
            insta::assert_snapshot!(format!("status_{which}_{width}x{height}"), screen);
        }
    }
}
#[test]
fn status_keys_preserve_escape_and_live_config_preserves_the_row() {
    let (mut state, plugins) = scenario("overview");
    let rows = rows::rows(&(&state).into(), &plugins, &[], None);
    put(&mut state, draw::tree(&rows, "permission", 80, 11, 0), 24);
    let (mut ui, now) = scene();
    ui.focus = crate::focus::Focus::Pane(format!("{}/bingo.mod.status/status", state.summary.id));
    draw_tree(80, 24, &solo(&state), &mut ui, now);
    for key in [KeyCode::Down, KeyCode::Up, KeyCode::Enter] {
        let effects = crate::input::on_key(
            &mut ui,
            &solo(&state),
            KeyEvent::new(key, KeyModifiers::NONE),
            now,
        );
        assert!(
            matches!(&effects[..], [crate::effect::Effect::SiteKey { action, .. }] if action.name == "bingo.mod.status:ui")
        );
    }
    let mut config = state.config.clone();
    config.plugins.insert(
        "bingo.permissions".into(),
        json!({"mode":"plan","rules":[]}),
    );
    state.apply(&frame(1, Event::ConfigChanged { config }));
    let rows = rows::rows(&(&state).into(), &plugins, &[], None);
    put(&mut state, draw::tree(&rows, "permission", 80, 11, 0), 24);
    let screen = draw_tree(80, 24, &solo(&state), &mut ui, now);
    assert!(screen.contains("❯ permission plan"), "{screen}");
    let escape = KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE);
    let effects = crate::input::on_key(&mut ui, &solo(&state), escape, now);
    assert!(
        matches!(&effects[..], [crate::effect::Effect::Submit(Input::Action { action, .. })] if action.args["event"] == "close")
    );
    // Exercise the live pulled site, not the snapshot test's pre-rendered tree.
    state
        .signals
        .get_mut("bingo.mod.status")
        .unwrap()
        .insert("status".into(), bingo_mod_status_slot().into());
    state.apply(&frame(2, started("running")));
    assert_eq!(
        crate::input::on_key(&mut ui, &solo(&state), escape, now),
        vec![crate::effect::Effect::Interrupt]
    );
}

fn bingo_mod_status_slot() -> serde_json::Value {
    slot::with(
        json!({}),
        [Slot::Pane {
            id: "status".into(),
            title: String::new(),
            focus: true,
            close_on_escape: true,
            hold_toasts: true,
            asked: true,
            place: slot::above(),
            size: PaneSize::from_rows(12),
            view: Shown::Pull {
                request: render::PullRequest {
                    component: render::Component::Status,
                    request_id: "status".into(),
                    props: json!({}),
                    viewport: render::Viewport {
                        place: None,
                        columns: 0,
                        rows: 0,
                    },
                },
            },
        }],
    )
}

#[test]
fn wake_row_counts_down_in_the_status_card_and_disappears() {
    for (width, height) in [(108, 30), (80, 24)] {
        let (mut state, plugins) = scenario("overview");
        let (mut ui, mut now) = scene();
        ui.focus =
            crate::focus::Focus::Pane(format!("{}/bingo.mod.status/status", state.summary.id));
        let pending = bingo_services::schedule::Pending {
            at: (now.wall + jiff::SignedDuration::from_mins(4)).to_string(),
            note: "check CI".into(),
        };
        for (seconds, expected) in [(0, "in 4m"), (20, "in 3m 40s")] {
            now.wall += jiff::SignedDuration::from_secs(seconds);
            let mut rows = rows::rows(&(&state).into(), &plugins, &[], None);
            rows.insert(3, rows::wake(Some(&pending), now.wall).unwrap());
            put(
                &mut state,
                draw::tree(&rows, "wake", width, (height - 4) / 2, 0),
                height,
            );
            let screen = draw_tree(width, height, &solo(&state), &mut ui, now);
            assert!(screen.contains(expected), "{screen}");
            assert!(screen.contains("/wake off"), "{screen}");
        }
        let rows = rows::rows(&(&state).into(), &plugins, &[], None);
        put(
            &mut state,
            draw::tree(&rows, "wake", width, (height - 4) / 2, 0),
            height,
        );
        assert!(!draw_tree(width, height, &solo(&state), &mut ui, now).contains("check CI"));
    }
}
