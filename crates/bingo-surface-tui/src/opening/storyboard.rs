//! Snapshot the opening at each beat.

use ratatui::text::Line;

use crate::painted::{in_look, truecolor};

/// The seconds the storyboard is read at: the light running, the light home,
/// the first row arriving, the mark settled, the last row home, and the box.
const AT: [f32; 5] = [0.0, 0.5, 1.2, 2.0, 2.4];

const WIDTHS: [u16; 1] = [80];

fn boxed(width: u16) -> Vec<Line<'static>> {
    crate::welcome::lines(&crate::test_support::state(), usize::from(width), None)
}

/// One frame as a person reads it off the screen. Each row is quoted, as the
/// screen catalogue's are, because half of what this piece does is leave a cell
/// *blank* and a snapshot drops the spaces off the end of a line.
fn drawn(t: f32, width: u16) -> String {
    super::frame(t, &boxed(width))
        .iter()
        .map(|row| format!("\"{row}\""))
        .collect::<Vec<_>>()
        .join("\n")
}

/// One frame's name for a file and a snapshot: `shot_1_1_80`.
fn named(t: f32, width: u16) -> String {
    format!("shot_{}_{width}", format!("{t:.1}").replace('.', "_"))
}

#[test]
fn the_five_beats_of_the_storyboard() {
    for t in AT {
        for width in WIDTHS {
            insta::assert_snapshot!(named(t, width), in_look(truecolor(), || drawn(t, width)));
        }
    }
}
