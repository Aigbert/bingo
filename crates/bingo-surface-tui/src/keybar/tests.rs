use super::*;
use crate::test_support::*;
use bingo_sdk::{View, component::KeyToast};
use ratatui::{Terminal, backend::TestBackend, widgets::Paragraph};

#[test]
fn approved_prototype_keybar_states() {
    let states: std::collections::BTreeMap<String, KeyBarProps> =
        serde_json::from_str(include_str!("prototype.json")).unwrap();
    for (name, props) in states {
        let mut screens = String::new();
        for width in [108, 80, 32] {
            let mut terminal = Terminal::new(TestBackend::new(width, 1)).unwrap();
            terminal
                .draw(|frame| {
                    frame.render_widget(
                        Paragraph::new(own(&Props::KeyBar(props.clone()), width as usize)),
                        frame.area(),
                    )
                })
                .unwrap();
            screens.push_str(&format!("{width} columns\n{}\n", terminal.backend()));
        }
        insta::assert_snapshot!(name, screens);
    }
}

#[test]
fn a_toast_replaces_the_previous_one_without_moving_the_frame() {
    let (mut ui, now) = scene();
    let tree = solo(&state());
    let before = draw_tree(108, 30, &tree, &mut ui, now);
    let regions = ui.shown.regions;
    ui.notify(bingo_sdk::Level::Info, "first", now.instant);
    draw_tree(108, 30, &tree, &mut ui, now);
    assert_eq!(ui.shown.regions.transcript, regions.transcript);
    assert_eq!(ui.shown.regions.composer, regions.composer);
    ui.notify(bingo_sdk::Level::Warn, "second", later(now, 100).instant);
    let screen = draw_tree(108, 30, &tree, &mut ui, later(now, 2899));
    assert!(screen.contains("second"));
    assert!(!screen.contains("first"));
    assert_eq!(draw_tree(108, 30, &tree, &mut ui, later(now, 2900)), before);
}

#[test]
fn keys_have_glow_actions_are_dim_and_separators_are_ash() {
    let props = Props::KeyBar(KeyBarProps {
        pairs: vec![KeyPair::new("↑↓", "pick"), KeyPair::new("esc", "close")],
        toast: None,
    });
    let line = own(&props, 80).remove(0);
    assert_eq!(line.spans[1].style, theme::glow().patch(theme::bold()));
    assert_eq!(line.spans[2].style, theme::dim());
    assert_eq!(line.spans[3].style, theme::ash());
}

#[test]
fn an_open_ask_seals_the_keybar_even_when_a_mod_claims_it() {
    let mut state = state();
    let ask = permission(None, None);
    state.interactions.push(ask.clone());
    state.config.kernel["render"] = serde_json::json!({"KeyBar":[{}]});
    let tree = solo(&state);
    let (mut ui, now) = scene();
    ui.dialog.focus_on(Some(&ask));
    let screen = draw_tree(108, 30, &tree, &mut ui, now);
    assert!(screen.contains("esc no, stop the turn"));
    assert!(
        !ui.shown
            .pulls
            .iter()
            .any(|key| key.request.component == bingo_sdk::render::Component::KeyBar)
    );
}

#[test]
fn the_keybar_is_replaceable_when_no_ask_is_open() {
    let mut state = state();
    state.config.kernel["render"] = serde_json::json!({"KeyBar":[{}]});
    let tree = solo(&state);
    let (mut ui, now) = scene();
    draw_tree(108, 30, &tree, &mut ui, now);
    let key = ui
        .shown
        .pulls
        .iter()
        .find(|key| key.request.component == bingo_sdk::render::Component::KeyBar)
        .unwrap()
        .clone();
    ui.placed.pulled.request(&key);
    ui.placed.pulled.answer(key, View::text("outside keybar"));
    assert!(draw_tree(108, 30, &tree, &mut ui, now).contains("outside keybar"));
}

#[test]
fn a_long_toast_wins_and_never_wraps() {
    let props = Props::KeyBar(KeyBarProps {
        pairs: vec![KeyPair::new("esc", "close")],
        toast: Some(KeyToast {
            about: None,
            text: "a long toast that exceeds the whole row\nand has another line".into(),
            level: bingo_sdk::Level::Info,
        }),
    });
    let lines = own(&props, 20);
    assert_eq!(lines.len(), 1);
    assert!(lines[0].width() <= 20);
    assert!(!lines[0].to_string().contains("esc"));
}

#[test]
fn an_expired_toast_never_uses_a_stale_mod_answer() {
    let mut state = state();
    state.config.kernel["render"] = serde_json::json!({"KeyBar":[{}]});
    let tree = solo(&state);
    let (mut ui, now) = scene();
    ui.notify(bingo_sdk::Level::Info, "gone forever", now.instant);
    draw_tree(108, 30, &tree, &mut ui, now);
    let key = ui
        .shown
        .pulls
        .iter()
        .find(|key| key.request.component == bingo_sdk::render::Component::KeyBar)
        .unwrap()
        .clone();
    ui.placed.pulled.request(&key);
    ui.placed.pulled.answer(key, View::text("gone forever"));
    assert!(!draw_tree(108, 30, &tree, &mut ui, later(now, 2800)).contains("gone forever"));
}

#[test]
fn the_keybar_reads_the_persons_keymap() {
    let mut state = state();
    state.config.kernel["keys"] = serde_json::json!({"ctrl+x":"/model next"});
    let (ui, now) = scene();
    let pairs = resting(&solo(&state), &ui, now);
    assert!(
        pairs
            .iter()
            .any(|pair| pair.keys == "ctrl+x" && pair.action == "next model")
    );
    assert!(
        !pairs
            .iter()
            .any(|pair| pair.keys == "alt+m" || pair.keys == "ctrl+p")
    );
}

#[test]
fn pane_seat_hints_reach_the_keybar_and_composer() {
    let mut state = state();
    state.signals.entry("outside".into()).or_default().insert("pane".into(), serde_json::json!({"slots":[{
        "kind":"pane","id":"one","title":"outside","focus":true,"closeOnEscape":true,"holdToasts":false,"asked":true,
        "hints":{"pairs":[{"keys":"q","action":"example","emphasis":false}],"composer":{"tag":"outside › ","placeholder":"choose"}},
        "view":{"kind":"Text","children":["body without hints"]}
    }]}).into());
    let tree = solo(&state);
    let (mut ui, now) = scene();
    ui.sync_panes(&state);
    let screen = draw_tree(108, 30, &tree, &mut ui, now);
    assert!(screen.contains("q example"));
    assert!(screen.contains("outside › choose"));
}

/// The lines the run opens with name a setting, and a setting cut short is no
/// remedy: each toast fits whole at 80 columns.
#[test]
fn the_opening_lines_fit_whole_at_eighty_columns() {
    for line in [
        crate::graphics::PASSTHROUGH_OFF,
        crate::graphics::PASSTHROUGH_UNHEARD,
        crate::graphics::unseen::HERDR_OFF,
    ] {
        let (mut ui, now) = scene();
        ui.notify(bingo_sdk::Level::Info, line, now.instant);
        let drawn = hints(&solo(&state()), &ui, 80, now, &mut Vec::new()).to_string();
        assert!(drawn.contains(line), "{drawn}");
    }
}
