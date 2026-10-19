use super::*;
use crate::test_support::*;
use bingo_sdk::View;

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
    let screen = draw_tree(108, 30, &tree, &mut ui, later(now, 2799));
    assert!(screen.contains("second"));
    assert!(!screen.contains("first"));
    assert_eq!(draw_tree(108, 30, &tree, &mut ui, later(now, 2800)), before);
}

#[test]
fn keys_are_muted_actions_are_dim_and_pairs_have_three_spaces() {
    let props = Props::KeyBar(KeyBarProps {
        pairs: vec![KeyPair::new("↑↓", "pick"), KeyPair::new("esc", "close")],
    });
    let line = own(&props, 80).remove(0);
    assert_eq!(line.spans[0].style, theme::foreground("muted").unwrap());
    assert_eq!(line.spans[1].style, theme::foreground("dim").unwrap());
    assert_eq!(line.spans[2].content, "   ");
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
    assert!(screen.contains("⏎ confirm   esc stop"), "{screen}");
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
fn an_expired_toast_never_uses_a_stale_mod_answer() {
    let mut state = state();
    state.config.kernel["render"] = serde_json::json!({"Toast":[{}]});
    let (mut ui, now) = scene();
    state.signals.entry("fixture".into()).or_default().insert(
        "toast".into(),
        bingo_sdk::slot::with(
            serde_json::json!({}),
            [bingo_sdk::slot::Slot::Toast {
                tone: bingo_sdk::Tone::Neutral,
                parts: vec!["gone forever".into()],
                until: later(now, 2700).wall,
                alert: false,
            }],
        )
        .into(),
    );
    let tree = solo(&state);
    ui.sync_toasts(&state, now);
    draw_tree(108, 30, &tree, &mut ui, now);
    let key = ui
        .shown
        .pulls
        .iter()
        .find(|key| key.request.component == bingo_sdk::render::Component::Toast)
        .unwrap()
        .clone();
    ui.placed.pulled.request(&key);
    ui.placed.pulled.answer(key, View::text("gone forever"));
    assert!(!draw_tree(108, 30, &tree, &mut ui, later(now, 2800)).contains("gone forever"));
}

#[test]
fn the_keybar_reads_the_persons_keymap() {
    let (ui, _) = scene();
    let pairs = resting(&solo(&state()), &ui);
    assert_eq!(
        pairs,
        vec![
            KeyPair::new("/", "commands"),
            KeyPair::new("@", "files"),
            KeyPair::new("?", "keys")
        ]
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
    assert!(screen.contains("outside ›  choose"));
}

/// The lines the run opens with name a setting, and a setting cut short is no
/// remedy: each toast fits whole at 80 columns.
#[test]
fn the_opening_lines_fit_whole_at_eighty_columns() {
    let (mut ui, now) = scene();
    ui.notify(
        bingo_sdk::Level::Info,
        crate::graphics::PASSTHROUGH_OFF,
        now.instant,
    );
    let drawn = draw_tree(80, 24, &solo(&state()), &mut ui, still(now));
    assert!(drawn.contains("/ commands"));
    assert_eq!(drawn.lines().count(), 24);
    assert!(drawn.lines().last().unwrap().contains("? keys"));
}

fn mod_toast(state: &mut bingo_sdk::SessionState, now: Now) {
    state.signals.entry("fixture".into()).or_default().insert(
        "toast".into(),
        bingo_sdk::slot::with(
            serde_json::json!({}),
            [bingo_sdk::slot::Slot::Toast {
                tone: bingo_sdk::Tone::Good,
                parts: vec![
                    bingo_sdk::pick::bold("counter", "fg").into(),
                    "saved".into(),
                ],
                until: later(now, 2700).wall,
                alert: false,
            }],
        )
        .into(),
    );
}

#[test]
fn a_mod_toast_draws_its_parts_with_their_own_ink() {
    let mut state = state();
    let (mut ui, now) = scene();
    mod_toast(&mut state, now);
    let tree = solo(&state);
    let buffer = drawn(108, 30, &tree, &mut ui, now);
    let screen = buffer.to_string();
    let row = row_carrying(&screen, "counter");
    let line: String = (0..108)
        .map(|x| buffer.buffer()[(x, row)].symbol().to_string())
        .collect();
    let at = line.find("counter").unwrap();
    let column = line[..at].chars().count() as u16;
    let cell = &buffer.buffer()[(column, row)];
    assert_eq!(cell.fg, theme::foreground("fg").unwrap().fg.unwrap());
    assert!(cell.modifier.contains(theme::bold().add_modifier));
    assert!(line.contains("saved"));
    assert!(line.contains(theme::glyphs::get("ok")));
}

#[test]
fn a_mod_can_replace_how_a_toast_is_drawn() {
    let mut state = state();
    state.config.kernel["render"] = serde_json::json!({"Toast":[{}]});
    let (mut ui, now) = scene();
    mod_toast(&mut state, now);
    let tree = solo(&state);
    draw_tree(108, 30, &tree, &mut ui, now);
    let key = ui
        .shown
        .pulls
        .iter()
        .find(|key| key.request.component == bingo_sdk::render::Component::Toast)
        .unwrap()
        .clone();
    assert_eq!(key.request.props["parts"][1], "saved");
    ui.placed.pulled.request(&key);
    ui.placed.pulled.answer(key, View::text("drawn by a mod"));
    assert!(draw_tree(108, 30, &tree, &mut ui, now).contains("drawn by a mod"));
}

fn crew_tree(mut frames: Vec<bingo_sdk::Frame>) -> Tree {
    frames.insert(0, child_frame(1, announced("reviewer")));
    folded_tree(frames)
}

fn pairs(pairs: Vec<KeyPair>) -> Vec<(String, String)> {
    pairs.into_iter().map(|p| (p.keys, p.action)).collect()
}

fn named(list: &[(&str, &str)]) -> Vec<(String, String)> {
    list.iter()
        .map(|(k, a)| (k.to_string(), a.to_string()))
        .collect()
}

#[test]
fn at_home_a_waiting_teammate_puts_ctrl_g_first() {
    let tree = crew_tree(vec![child_frame(2, opened(child_permission()))]);
    let (ui, _) = scene();
    assert_eq!(
        pairs(resting(&tree, &ui)),
        named(&[("ctrl+g", "answer"), ("@", "files"), ("?", "keys")])
    );
}

#[test]
fn away_from_home_esc_is_back_and_ctrl_c_is_stop() {
    let mut tree = crew_tree(vec![child_frame(2, started("trn_9"))]);
    tree.show(&child_id());
    let (mut ui, now) = scene();
    assert_eq!(
        pairs(resting(&tree, &ui)),
        named(&[("esc", "back"), ("ctrl+c", "stop"), ("/", "commands")])
    );
    write(&mut ui, tree.viewed(), "x", now);
    assert_eq!(
        pairs(resting(&tree, &ui)),
        named(&[("⏎", "steer"), ("tab", "queue"), ("esc", "clear")])
    );
}
