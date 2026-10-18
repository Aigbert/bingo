//! Turn snapshots keep every step in order, with no turn-fold holder.
use super::*;

fn running() -> (Tree, Ui, Now) {
    let (ui, now) = scene();
    (solo(&folded(running_turn())), ui, still(later(now, 7_500)))
}
fn finished() -> (Tree, Ui, Now) {
    let (ui, now) = scene();
    (solo(&folded(finished_turn())), ui, later(now, 9_000))
}
fn stopped() -> (Tree, Ui, Now) {
    let (ui, now) = scene();
    (solo(&folded(stopped_turn())), ui, later(now, 9_000))
}
fn expanded() -> (Tree, Ui, Now) {
    let (mut ui, now) = scene();
    let state = folded(finished_turn());
    open_the_turns(&mut ui, &state);
    (solo(&state), ui, later(now, 9_000))
}

#[test]
fn turn_running() {
    let (tree, mut ui, now) = running();
    let screen = draw_tree(80, 24, &tree, &mut ui, now);
    for row in [
        "   ¶ read    src/main.rs",
        "   / search  total",
        "   ◌ run     cargo test -q",
    ] {
        assert!(screen.contains(row), "{screen}");
    }
    assert!(!screen.contains("printed bare"));
    both("turn_running", &tree, &mut ui, now);
}
#[test]
fn turn_finished() {
    let (tree, mut ui, now) = finished();
    let screen = draw_tree(80, 24, &tree, &mut ui, now);
    for text in [
        "read    src/main.rs",
        "search  total",
        "edit    src/main.rs",
        "run     cargo test -q",
        "One place prints it.",
        "Done. main now prints",
    ] {
        assert!(screen.contains(text), "{screen}");
    }
    assert!(!screen.contains("4 steps"));
    both("turn_finished", &tree, &mut ui, now);
}
#[test]
fn turn_stopped() {
    let (tree, mut ui, now) = stopped();
    let screen = draw_tree(80, 24, &tree, &mut ui, now);
    assert!(screen.contains("stopped"));
    assert!(!screen.contains("you stopped the turn"));
    both("turn_stopped", &tree, &mut ui, now);
}
#[test]
fn turn_expanded() {
    let (tree, mut ui, now) = expanded();
    let screen = draw_tree(120, 40, &tree, &mut ui, now);
    assert!(screen.contains("   ± edit    src/main.rs"));
    assert!(screen.contains("     test result: ok. 3 passed"));
    assert!(!screen.contains("/tmp/project/src"));
    both("turn_expanded", &tree, &mut ui, now);
}
#[test]
fn turns_many() {
    let (mut ui, now) = scene();
    both(
        "turns_many",
        &solo(&folded(long_session(8))),
        &mut ui,
        later(now, 9_000),
    );
}
#[test]
fn ascii_turns() {
    let (tree, mut ui, now) = expanded();
    without_glyphs("ascii_turn_expanded", &tree, &mut ui, now);
    let (tree, mut ui, now) = stopped();
    without_glyphs("ascii_turn_stopped", &tree, &mut ui, now);
}
#[test]
fn running_steps_do_not_repeat_the_footer_action() {
    let (_, mut ui, now) = running();
    let state = crate::transcript::slots::fixtures::with_promotions(folded(running_turn()));
    let screen = draw_tree(80, 24, &solo(&state), &mut ui, now);
    assert!(screen.contains("   ◌ run     cargo test -q"));
    assert!(!screen.contains("[ b background ]"));
}
#[test]
fn an_ended_step_hangs_no_action() {
    let (_, mut ui, now) = expanded();
    let state = crate::transcript::slots::fixtures::with_promotions(folded(finished_turn()));
    open_the_turns(&mut ui, &state);
    let screen = draw_tree(120, 40, &solo(&state), &mut ui, now);
    assert!(screen.contains("   $ run     cargo test -q"));
    assert!(!screen.contains("background"));
}
#[test]
fn ctrl_o_opens_only_the_latest_steps_output() {
    let (tree, mut ui, now) = finished();
    draw_tree(80, 24, &tree, &mut ui, now);
    crate::input::on_key(&mut ui, &tree, ctrl('o'), now);
    let screen = draw_tree(80, 24, &tree, &mut ui, now);
    assert!(screen.contains("test result: ok. 3 passed"));
    assert!(screen.contains("edit    src/main.rs"));
    assert!(!screen.contains("4 steps"));
}
#[test]
fn the_builtin_ledger_draws_in_ascii_where_the_terminal_does() {
    let (tree, mut ui, now) = finished();
    let mut state = tree.viewed().clone();
    state.config.kernel["render"] = serde_json::json!({"Ledger": [{}]});
    let tree = solo(&state);
    let draw = |ui: &mut Ui| in_look(ascii(), || draw_tree(80, 24, &tree, ui, now));
    draw(&mut ui);
    let keys = ui.shown.pulls.clone();
    assert_eq!(keys.len(), 4);
    for key in keys {
        let width = usize::from(key.request.viewport.columns);
        let props =
            bingo_sdk::component::Props::parse(key.request.component, key.request.props.clone())
                .unwrap();
        let bingo_sdk::component::Props::Ledger(props) = props else {
            panic!("Ledger only");
        };
        let view = bingo_mod_ledger::draw::ledger(&props, width);
        ui.placed.pulled.request(&key);
        ui.placed.pulled.answer(key, view);
    }
    let screen = draw(&mut ui);
    assert!(screen.contains("   < read    src/main.rs"));
    assert!(screen.contains("   ~ edit    src/main.rs"));
}
