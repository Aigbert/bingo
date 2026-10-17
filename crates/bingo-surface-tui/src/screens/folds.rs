//! The screens a turn that did some work is read through (ADR-0095 §6–§8):
//! running, finished, stopped, opened, and a long run of them.
//!
//! One thing is decided here and nowhere else: the transcript's left edge.
//! Nothing is drawn at column 0; a prompt's `›`, a reply's `▍` and a
//! notice's `⏺` sit at column 1; a step, a turn's folded line and its stop line start at column 3;
//! and whatever a row says at its right ends two cells before the edge.

use super::*;

/// What a part of a turn starts with at column 3, in either state of a fold.
const PARTS: [char; 2] = ['⏺', '■'];

/// The rows between the header and the composer's rule.
fn transcript(screen: &str) -> Vec<Vec<char>> {
    screen
        .lines()
        .skip(1)
        .take_while(|row| !row.contains('─'))
        .map(|row| row.trim_matches('"').chars().collect())
        .collect()
}

/// The left edge every row of a transcript keeps, and the right edge every
/// part of a turn keeps.
fn assert_aligned(screen: &str, width: usize) {
    for row in transcript(screen) {
        let text: String = row.iter().collect();
        assert_eq!(row.first(), Some(&' '), "nothing at column 0: {text:?}");
        let Some(first) = row.iter().position(|c| *c != ' ') else {
            continue;
        };
        match first {
            1 => assert!(matches!(row[1], '›' | '▍' | '⏺'), "column 1: {text:?}"),
            3 => assert!(PARTS.contains(&row[3]), "column 3: {text:?}"),
            _ => assert!(first >= 5, "under a step's words: {text:?}"),
        }
        let last = row.iter().rposition(|c| *c != ' ').unwrap_or_default();
        assert!(last <= width - 3, "two cells from the edge: {text:?}");
    }
}

/// The folded line's elapsed time sits flush against the right margin.
fn assert_flush(screen: &str, width: usize) {
    let lines = transcript(screen);
    let folded = lines
        .iter()
        .find(|row| row.get(3) == Some(&'⏺') && row.iter().collect::<String>().contains(" step"));
    let Some(row) = folded else {
        panic!("a folded line: {screen}");
    };
    assert_eq!(row.iter().rposition(|c| *c != ' '), Some(width - 3));
}

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
fn every_row_of_a_turn_keeps_the_same_left_edge() {
    for (scene, (tree, mut ui, now)) in [
        ("running", running()),
        ("finished", finished()),
        ("stopped", stopped()),
        ("expanded", expanded()),
    ] {
        for (width, height) in [(80, 32), (120, 40)] {
            let screen = draw_tree(width, height, &tree, &mut ui, now);
            assert!(screen.contains(" › label the total"), "{scene}: {screen}");
            assert_aligned(&screen, usize::from(width));
            if scene != "running" {
                assert_flush(&screen, usize::from(width));
            }
        }
    }
}

/// The steps one row each, the one running a bullet that breathes, no
/// thought a row.
#[test]
fn turn_running() {
    let (tree, mut ui, now) = running();
    let screen = draw_tree(80, 24, &tree, &mut ui, now);
    for row in [
        "   ⏺ read src/main.rs",
        "   ⏺ grep total",
        "   ⏺ bash cargo test -q",
    ] {
        assert!(screen.contains(row), "{row}: {screen}");
    }
    assert!(!screen.contains("printed bare"), "no thought: {screen}");
    both("turn_running", &tree, &mut ui, now);
}

/// One line for the work, the answer under it.
#[test]
fn turn_finished() {
    let (tree, mut ui, now) = finished();
    let screen = draw_tree(80, 24, &tree, &mut ui, now);
    assert!(
        screen.contains("   ⏺ 4 steps · read · grep · edit +1 −1 · bash 3 passed"),
        "{screen}"
    );
    assert!(!screen.contains("One place prints it."), "{screen}");
    assert!(screen.contains(" ▍ Done. main now prints"), "{screen}");
    both("turn_finished", &tree, &mut ui, now);
}

/// The line says it was stopped; the line under it says how far it got.
#[test]
fn turn_stopped() {
    let (tree, mut ui, now) = stopped();
    let screen = draw_tree(80, 24, &tree, &mut ui, now);
    assert!(screen.contains(" · stopped"), "{screen}");
    assert!(
        screen.contains("   ■ you stopped the turn · 3 of 4 steps had run; nothing after them"),
        "{screen}"
    );
    both("turn_stopped", &tree, &mut ui, now);
}

/// Opened, the steps stand under the line in the same columns, what they
/// brought back under their words, the paths read from the project.
#[test]
fn turn_expanded() {
    let (tree, mut ui, now) = expanded();
    let screen = draw_tree(120, 40, &tree, &mut ui, now);
    assert!(screen.contains("   ⏺ 4 steps"), "{screen}");
    assert!(screen.contains("   ⏺ edit src/main.rs"), "{screen}");
    assert!(
        screen.contains("     test result: ok. 3 passed"),
        "{screen}"
    );
    assert!(!screen.contains("/tmp/project/src"), "relative: {screen}");
    both("turn_expanded", &tree, &mut ui, now);
}

/// Many turns of work: one line each, the answers between.
#[test]
fn turns_many() {
    let (mut ui, now) = scene();
    let tree = solo(&folded(long_session(8)));
    let screen = draw_tree(80, 24, &tree, &mut ui, later(now, 9_000));
    assert_aligned(&screen, 80);
    both("turns_many", &tree, &mut ui, later(now, 9_000));
}

/// Where only ASCII draws, the same columns in the ASCII table's marks.
#[test]
fn ascii_turns() {
    let (tree, mut ui, now) = expanded();
    without_glyphs("ascii_turn_expanded", &tree, &mut ui, now);
    let (tree, mut ui, now) = stopped();
    without_glyphs("ascii_turn_stopped", &tree, &mut ui, now);
}

/// A producer that only offers an action on a running step draws nothing of
/// its own: the step keeps its row, the action hangs under its words.
#[test]
fn an_action_on_a_running_step_hangs_under_it() {
    let (_, mut ui, now) = running();
    let state = crate::transcript::slots::fixtures::with_promotions(folded(running_turn()));
    let screen = draw_tree(80, 24, &solo(&state), &mut ui, now);
    assert!(screen.contains("   ⏺ bash cargo test -q"), "{screen}");
    assert!(screen.contains("     [ b background ]"), "{screen}");
    assert_aligned(&screen, 80);
}

/// A step that has ended offers nothing, even where its producer has not
/// yet taken the action back.
#[test]
fn an_ended_step_hangs_no_action() {
    let (_, mut ui, now) = expanded();
    let state = crate::transcript::slots::fixtures::with_promotions(folded(finished_turn()));
    open_the_turns(&mut ui, &state);
    let screen = draw_tree(120, 40, &solo(&state), &mut ui, now);
    assert!(screen.contains("   ⏺ bash cargo test -q"), "{screen}");
    assert!(!screen.contains("background"), "{screen}");
}

/// `ctrl+o` opens the latest folded turn where it stands.
#[test]
fn ctrl_o_opens_the_latest_folded_turn() {
    let (tree, mut ui, now) = finished();
    draw_tree(80, 32, &tree, &mut ui, now);
    crate::input::on_key(&mut ui, &tree, ctrl('o'), now);
    let screen = draw_tree(80, 32, &tree, &mut ui, now);
    assert!(screen.contains("   ⏺ 4 steps"), "{screen}");
    assert!(screen.contains("   ⏺ edit src/main.rs"), "{screen}");
}

/// A click on the folded line opens it and a second folds it again; `⏎` on
/// the line the click left held opens it once more.
#[test]
fn a_click_or_enter_on_the_folded_line_opens_it() {
    let (tree, mut ui, now) = finished();
    let line = |ui: &mut Ui| {
        let screen = draw_tree(80, 32, &tree, ui, now);
        (row_carrying(&screen, "4 steps"), screen)
    };
    let (row, _) = line(&mut ui);
    crate::pointer::on_mouse(&mut ui, &tree, click(5, row), now);
    let (row, opened) = line(&mut ui);
    assert!(opened.contains("   ⏺ 4 steps"), "{opened}");
    assert!(opened.contains("   ⏺ edit src/main.rs"), "{opened}");

    crate::pointer::on_mouse(&mut ui, &tree, click(5, row), now);
    let (_, shut) = line(&mut ui);
    assert!(shut.contains("   ⏺ 4 steps"), "{shut}");
    assert!(!shut.contains("⏺ edit src/main.rs"), "{shut}");

    crate::input::on_key(&mut ui, &tree, key(crossterm::event::KeyCode::Enter), now);
    let (_, again) = line(&mut ui);
    assert!(again.contains("   ⏺ 4 steps"), "{again}");
}

/// Under an opened turn a click is where a selection starts: only the line
/// the turn folds to shuts it.
#[test]
fn a_click_under_an_opened_turn_leaves_it_open() {
    let (tree, mut ui, now) = finished();
    let screen = draw_tree(80, 32, &tree, &mut ui, now);
    crate::pointer::on_mouse(
        &mut ui,
        &tree,
        click(5, row_carrying(&screen, "4 steps")),
        now,
    );
    let opened = draw_tree(80, 32, &tree, &mut ui, now);
    for under in ["edit src/main.rs", "println!"] {
        let row = row_carrying(&opened, under);
        crate::pointer::on_mouse(&mut ui, &tree, click(8, row), now);
        crate::pointer::on_mouse(&mut ui, &tree, dragged(20, row), now);
        crate::pointer::on_mouse(&mut ui, &tree, released(20, row), now);
        let still = draw_tree(80, 32, &tree, &mut ui, now);
        assert!(
            still.contains("   ⏺ 4 steps"),
            "a click on {under:?}: {still}"
        );
    }
}

/// The built-in ledger answers in this terminal's marks: under ASCII, a
/// turn's rows are ASCII whoever draws them.
#[test]
fn the_builtin_ledger_draws_in_ascii_where_the_terminal_does() {
    let (tree, mut ui, now) = expanded();
    let mut state = tree.viewed().clone();
    state.config.kernel["render"] = serde_json::json!({"Ledger": [{}], "TurnFold": [{}]});
    let tree = solo(&state);
    let draw = |ui: &mut Ui| in_look(ascii(), || draw_tree(80, 32, &tree, ui, now));
    draw(&mut ui);
    let keys = ui.shown.pulls.clone();
    assert!(keys.len() > 1, "the line and its steps ask: {keys:?}");
    for key in keys {
        let width = usize::from(key.request.viewport.columns);
        let props =
            bingo_sdk::component::Props::parse(key.request.component, key.request.props.clone());
        let view = match props.expect("typed props") {
            bingo_sdk::component::Props::Ledger(props) => {
                bingo_mod_ledger::draw::ledger(&props, width)
            }
            bingo_sdk::component::Props::TurnFold(props) => {
                bingo_mod_ledger::draw::fold(&props, width)
            }
            other => panic!("only the ledger's sites ask: {other:?}"),
        };
        ui.placed.pulled.request(&key);
        ui.placed.pulled.answer(key, view);
    }
    let screen = draw(&mut ui);
    assert!(screen.contains("   * 4 steps"), "{screen}");
    assert!(screen.contains("   * read src/main.rs"), "{screen}");
    for mark in ['⏺', '▍'] {
        assert!(!screen.contains(mark), "{mark} under ASCII: {screen}");
    }
}
