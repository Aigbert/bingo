//! Rewind uses the shared sheet window. An ask in the dock keeps its last
//! rows as the prototype's `body.slice(-cap)` does (M197).

use super::*;

use crate::rewind::Rewind;

/// One picker at 80×24, the size every card must be readable at (§7).
fn shot(name: &str, tree: &Tree, ui: &mut Ui, now: Now) -> String {
    let screen = draw_tree(80, 24, tree, ui, now);
    insta::assert_snapshot!(name, screen.clone());
    screen
}

/// A transcript of fourteen turns, so the rewind card has more of them to list
/// than its eight rows.
fn crowded_rewind(selected: usize) -> (bingo_sdk::SessionState, Ui, Now) {
    let mut state = state();
    state.transcript = (1..=14)
        .map(|i| {
            let mut asked = user(&format!("itm_{i}"), &format!("turn {i:02}: do the thing"));
            asked.turn = Some(TurnId::from_raw(format!("trn_{i}")));
            asked
        })
        .collect();
    let (mut ui, now) = scene();
    shown(
        &mut ui,
        Open::Rewind(Rewind {
            selected: Some(TurnId::from_raw(format!("trn_{}", 14 - selected))),
            ..Default::default()
        }),
        now,
    );
    (state, ui, now)
}

#[test]
fn the_rewind_card_scrolls_under_the_row_the_keyboard_is_on() {
    let (state, mut ui, now) = crowded_rewind(13);
    let screen = shot("rewind_end", &solo(&state), &mut ui, now);
    assert!(
        screen
            .lines()
            .any(|row| row.contains('▎') && row.contains("turn 01"))
    );
    assert!(screen.contains("7 of 14"));
    assert!(screen.contains("Rewind"), "the title stays: {screen}");

    let (state, mut ui, now) = crowded_rewind(6);
    let screen = shot("rewind_middle", &solo(&state), &mut ui, now);
    assert!(
        screen
            .lines()
            .any(|row| row.contains('▎') && row.contains("turn 08"))
    );
    assert!(screen.contains("7 of 14"));
}
