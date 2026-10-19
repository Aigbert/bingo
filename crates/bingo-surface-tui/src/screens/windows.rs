//! Rewind uses the shared sheet window; ask options keep their older cut marks.

use super::*;

use crate::rewind::Rewind;

/// One picker at 80×24, the size every card must be readable at (§7).
fn shot(name: &str, tree: &Tree, ui: &mut Ui, now: Now) -> String {
    let screen = draw_tree(80, 24, tree, ui, now);
    insta::assert_snapshot!(name, screen.clone());
    screen
}

/// The one row the keyboard is on, as it was drawn.
fn marked(screen: &str) -> String {
    let rows: Vec<&str> = screen.lines().filter(|row| row.contains('❯')).collect();
    assert_eq!(rows.len(), 1, "one cursor on the screen: {rows:?}");
    rows[0].trim().to_string()
}

/// How many ends of the list said they were cut short. A drawn row comes
/// quoted, as the backend prints it, and a card's rows come between its own
/// two uprights.
fn cuts(screen: &str) -> usize {
    screen
        .lines()
        .filter(|row| row.replace(['"', '│'], "").trim() == "…")
        .count()
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

/// A question with a dozen options, each with a description: a card with more
/// answers in it than the box has rows to walk them in.
fn crowded_question(focus: usize) -> (Tree, Ui, Now) {
    let mut open = question(false, false);
    if let bingo_sdk::InteractionKind::Question(bingo_sdk::Question { options, .. }) =
        &mut open.kind
    {
        *options = (1..=12)
            .map(|i| bingo_sdk::QuestionOption {
                id: format!("o{i}"),
                label: format!("option {i:02}"),
                description: Some(format!("what option {i:02} means")),
                role: None,
                preview: None,
            })
            .collect();
    }
    let (tree, mut ui, now) = super::asked(open);
    ui.dialog.focus = focus;
    (tree, ui, now)
}

/// A form of three questions whose first has `options` of them and a mockup on
/// each: the tab row, the answers and the one under the cursor all want the
/// same short screen.
fn crowded_form(options: usize, focus: usize) -> (Tree, Ui, Now) {
    let mut open = crate::test_support::form();
    if let bingo_sdk::InteractionKind::Form { questions, .. } = &mut open.kind {
        questions[0].options = (1..=options)
            .map(|i| bingo_sdk::QuestionOption {
                id: format!("o{i}"),
                label: format!("option {i:02}"),
                description: Some(format!("what option {i:02} means")),
                role: None,
                preview: Some(format!("a mockup of {i:02}\nover two rows")),
            })
            .collect();
    }
    let (tree, mut ui, now) = super::asked(open);
    ui.dialog.focus = focus;
    (tree, ui, now)
}

/// The tab row is the card's first line, so it is the row [`layers::card`]
/// keeps whatever else gives way. On a screen this short the card gives away
/// the key line and then the mockup — an answer is what it was opened for
/// (M57) — and every answer, the one under the cursor included, stays.
#[test]
fn a_form_keeps_its_tabs_and_the_answer_the_cursor_is_on() {
    let (tree, mut ui, now) = crowded_form(12, 11);
    let screen = shot("form_end", &tree, &mut ui, now);
    assert!(marked(&screen).contains("option 12"), "{screen}");
    assert_eq!(
        cuts(&screen),
        0,
        "the taller stream fits all twelve options: {screen}"
    );
    assert!(
        !screen.contains("Enter to select"),
        "the keys gave way first: {screen}"
    );
    assert!(
        !screen.contains("a mockup of"),
        "and the mockup whole, never half a frame: {screen}"
    );
    assert!(screen.contains("Auth method"), "the tabs stay: {screen}");
    assert!(screen.contains("Targets"), "all of them: {screen}");
}

/// More answers than the room even with the keys and the mockup given away:
/// the answers keep their window around the cursor and say where they cut it.
#[test]
fn a_form_with_more_answers_than_room_windows_them_around_the_cursor() {
    let (tree, mut ui, now) = crowded_form(24, 23);
    let screen = shot("form_crowded", &tree, &mut ui, now);
    assert!(marked(&screen).contains("option 24"), "{screen}");
    assert_eq!(cuts(&screen), 1);
    assert!(screen.contains("Auth method"), "the tabs stay: {screen}");
}

/// The answers are what the card was opened for, so they are what keeps the
/// room: the preview above them still gives way first (§10, 2026-08-31).
#[test]
fn a_card_walks_its_answers_without_losing_the_one_it_is_on() {
    let (tree, mut ui, now) = crowded_question(0);
    let screen = shot("question_head", &tree, &mut ui, now);
    assert!(marked(&screen).contains("option 01"), "{screen}");
    assert_eq!(cuts(&screen), 1);
    assert!(screen.contains("Auth"), "the title stays: {screen}");

    let (tree, mut ui, now) = crowded_question(11);
    let screen = shot("question_end", &tree, &mut ui, now);
    assert!(marked(&screen).contains("option 12"), "{screen}");
    assert_eq!(cuts(&screen), 1);
}
