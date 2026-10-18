//! What a block does as it lands (design §6), on the same injected clock as
//! every other row: the light that crosses the name of a call that came back,
//! the cooling of one that came back wrong, and the room a new block takes —
//! which is none but its own.

use bingo_sdk::{Event, ItemStatus};
use ratatui::style::Style;

use super::screen;
use crate::clock::Now;
use crate::test_support::*;
use crate::theme;
use crate::tree::Tree;

/// A tool call that has just come back, with the cache warm on the frame
/// before — which is what makes it a completion a person watched, rather than
/// a row that was already there. Outside any turn, so it is its own row: a
/// turn's step lands as its step row does.
fn just_landed(ok: bool, ui: &mut crate::ui::Ui, now: Now) -> Tree {
    let mut state = folded(vec![
        frame(1, started("trn_1")),
        started_tool(2, loose(running_tool("itm_1", "bash", "compiling…"))),
    ]);
    crate::painted::painted(80, 24, &solo(&state), ui, now);
    let output = match ok {
        true => bingo_sdk::ToolOutput::text("ok"),
        false => bingo_sdk::ToolOutput {
            is_error: true,
            ..bingo_sdk::ToolOutput::text("exit 1")
        },
    };
    state.apply(&frame(
        3,
        Event::ItemCompleted {
            item: loose(tool(
                "itm_1",
                "bash",
                serde_json::json!({"command": "cargo test"}),
                Some(output),
                match ok {
                    true => ItemStatus::Completed,
                    false => ItemStatus::Failed,
                },
            )),
        },
    ));
    solo(&state)
}

/// The runs of the row a call landed on, so the light crossing its name can
/// be told from the one style it rests in.
fn landed_runs(tree: &Tree, ui: &mut crate::ui::Ui, now: Now) -> Vec<(String, Style)> {
    crate::painted::painted(80, 24, tree, ui, now).row("bash")
}

/// The one bold frame a completion used to wear lasted 33 ms, which is below
/// the threshold at which a person reads it as *something happening*. One
/// light crosses the row's name instead, over six frames, and the row is at
/// rest on the seventh — where "at rest" is exactly the row it always was.
#[test]
fn a_completion_lands_only_its_mark_for_420_ms() {
    let (mut ui, now) = scene();
    let done = just_landed(true, &mut ui, now);
    theme::with(crate::painted::truecolor(), || {
        let fresh = landed_runs(&done, &mut ui, now);
        let halfway = landed_runs(&done, &mut ui, later(now, 210));
        let settled = landed_runs(&done, &mut ui, later(now, 420));
        assert_ne!(fresh, halfway);
        assert_ne!(halfway, settled);
        assert_eq!(settled, landed_runs(&done, &mut ui, still(later(now, 420))));
    });
}

#[test]
fn a_failure_keeps_its_bad_verb_after_the_mark_lands() {
    let (mut ui, now) = scene();
    let failed = just_landed(false, &mut ui, now);
    theme::with(crate::painted::truecolor(), || {
        for at in [now, later(now, 210), later(now, 420)] {
            let runs = landed_runs(&failed, &mut ui, at);
            assert!(
                runs.iter().any(|(text, style)| text.contains("bash")
                    && *style == theme::as_drawn(theme::bad()))
            );
        }
    });
}

fn row_of(screen: &str, needle: &str) -> usize {
    screen
        .lines()
        .position(|line| line.contains(needle))
        .unwrap_or_else(|| panic!("no row carries {needle:?}"))
}

#[test]
fn a_new_block_takes_its_own_room_and_walks_nowhere_after_it() {
    let (mut ui, now) = mid_turn();
    let mut state = folded(vec![frame(
        1,
        Event::ItemCompleted {
            item: user("itm_1", "first"),
        },
    )]);
    // The cache is warm after this draw, so the next block is one a person
    // watches arrive.
    let settled = screen(&solo(&state), &mut ui, now);
    let was = row_of(&settled, "first");

    state.apply(&frame(
        2,
        Event::ItemCompleted {
            item: assistant("itm_2", "second", ItemStatus::Completed),
        },
    ));
    let arriving = solo(&state);
    let frames: Vec<String> = [0i64, 33, 66, 99]
        .iter()
        .map(|ms| screen(&arriving, &mut ui, later(now, *ms)))
        .collect();
    assert_eq!(
        row_of(&frames[0], "first"),
        was - 2,
        "the answer and its preceding blank row grow short transcripts grow upward from the dock"
    );
    for (at, drawn) in frames.iter().enumerate().skip(1) {
        assert_eq!(
            drawn, &frames[0],
            "frame {at} of the arrival draws the same screen"
        );
    }
}
