//! Why a terminal that speaks the protocol still gets the chip.
//!
//! A terminal that says nothing about pictures has nothing a person could
//! change, and is told nothing. One that answered the kitty query `OK` and
//! is drawn chips all the same is a surprise worth one line: either its name
//! is not one known to draw the placeholder cells ([`super::placeholders`]),
//! or it never said how big a cell is. Under herdr the second has a remedy,
//! and the line names it.

use super::Probe;
use super::placeholders::{FORWARDS, draws_placeholders};
use super::probe::Named;
use super::tmux;

/// A terminal that said `OK` and no name at all.
pub const UNNAMED: &str = "pictures off: the terminal did not say its name";

/// A terminal known to draw pictures that never said how big a cell is.
pub const NO_CELL: &str = "pictures off: the terminal did not say its cell size";

/// herdr with pictures off: its pane says `OK` and `libghostty`, and gives
/// the pane a window of no pixels and no `CSI 16 t` answer. herdr 0.9 turned
/// the setting on by default.
pub const HERDR_OFF: &str =
    "herdr: pictures need `[experimental] kitty_graphics = true` (or herdr ≥ 0.9)";

/// The most of a terminal's own name a line repeats.
const LONGEST_NAME: usize = 32;

/// The one line there is to say about a terminal that answered `OK` and draws
/// no pictures, or nothing when it did not answer `OK` or does draw them.
///
/// A terminal on the list that has a cell, and is still drawn chips, is behind
/// a tmux too old to carry them — the route, not the terminal, and not this
/// line's to explain.
pub fn unseen(probe: &Probe) -> Option<String> {
    if !probe.kitty {
        return None;
    }
    match probe
        .terminals
        .iter()
        .find(|named| draws_placeholders(named))
    {
        Some(drawer) => probe.cell.is_none().then(|| no_cell(drawer).to_string()),
        None => Some(unknown(probe)),
    }
}

/// Why a terminal known to draw pictures draws none: no cell to draw into.
fn no_cell(drawer: &Named) -> &'static str {
    match drawer.name.eq_ignore_ascii_case(FORWARDS) {
        true => HERDR_OFF,
        false => NO_CELL,
    }
}

/// The terminal that answered, named, when it is not one known to draw the
/// cells. tmux answers for itself and is never the terminal meant: the one
/// behind it is.
fn unknown(probe: &Probe) -> String {
    let said = probe
        .terminals
        .iter()
        .rev()
        .find(|named| !tmux::named(named));
    match said.map(|named| shown(&named.name)) {
        Some(name) if !name.is_empty() => {
            format!("pictures off: {name} is not known to draw them")
        }
        _ => UNNAMED.to_string(),
    }
}

/// A name as a line may carry it: the terminal's own bytes, so nothing that
/// would steer the screen and nothing longer than a name.
fn shown(name: &str) -> String {
    name.chars()
        .filter(|c| !c.is_control())
        .take(LONGEST_NAME)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::graphics::Cell;

    const CELL: Cell = Cell {
        width: 10,
        height: 20,
    };

    fn probe(kitty: bool, cell: Option<Cell>, names: &[(&str, &str)]) -> Probe {
        Probe {
            kitty,
            cell,
            terminals: names
                .iter()
                .map(|(name, version)| Named {
                    name: (*name).into(),
                    version: (*version).into(),
                })
                .collect(),
        }
    }

    fn said(kitty: bool, cell: Option<Cell>, names: &[(&str, &str)]) -> Option<String> {
        unseen(&probe(kitty, cell, names))
    }

    #[test]
    fn a_terminal_that_never_said_ok_is_told_nothing() {
        assert_eq!(said(false, Some(CELL), &[("foot", "1.28.0")]), None);
        assert_eq!(said(false, None, &[]), None);
    }

    #[test]
    fn a_terminal_that_draws_pictures_is_told_nothing() {
        assert_eq!(said(true, Some(CELL), &[("ghostty", "1.3.1")]), None);
        assert_eq!(said(true, Some(CELL), &[("libghostty", "")]), None);
    }

    /// The name off the list is the one the line carries — the terminal's,
    /// never tmux's.
    #[test]
    fn a_name_off_the_list_is_named() {
        assert_eq!(
            said(true, Some(CELL), &[("WezTerm", "20240203-110809-5046fc22")]).as_deref(),
            Some("pictures off: WezTerm is not known to draw them")
        );
        assert_eq!(
            said(
                true,
                Some(CELL),
                &[("tmux", "3.6b"), ("Konsole", "26.08.0")]
            )
            .as_deref(),
            Some("pictures off: Konsole is not known to draw them")
        );
        assert_eq!(
            said(true, Some(CELL), &[("kitty", "0.27.9")]).as_deref(),
            Some("pictures off: kitty is not known to draw them"),
            "below its floor is off the list"
        );
    }

    #[test]
    fn a_terminal_that_said_ok_and_no_name_is_told_so() {
        assert_eq!(said(true, Some(CELL), &[]).as_deref(), Some(UNNAMED));
        assert_eq!(
            said(true, Some(CELL), &[("tmux", "3.6b")]).as_deref(),
            Some(UNNAMED),
            "tmux's own name is not the terminal's"
        );
    }

    /// herdr with pictures off answers just as with them on, bar the window:
    /// the line names the setting. Any other terminal is told the cell.
    #[test]
    fn no_cell_names_herdrs_setting_under_herdr_and_the_cell_elsewhere() {
        assert_eq!(
            said(true, None, &[("libghostty", "")]).as_deref(),
            Some(HERDR_OFF)
        );
        assert_eq!(
            said(true, None, &[("ghostty", "1.3.1")]).as_deref(),
            Some(NO_CELL)
        );
    }

    /// A tmux too old to carry the cells is the route's failing, not this
    /// line's.
    #[test]
    fn a_tmux_too_old_is_not_this_lines_to_explain() {
        assert_eq!(
            said(true, Some(CELL), &[("tmux", "3.3a"), ("ghostty", "1.3.1")]),
            None
        );
    }

    /// The name is the terminal's own bytes: a control byte in it would steer
    /// the screen the line is drawn on.
    #[test]
    fn a_name_is_shown_without_control_bytes_and_cut_short() {
        assert_eq!(shown("Wez\x1b[2JTerm"), "Wez[2JTerm");
        assert_eq!(shown(&"x".repeat(100)).len(), LONGEST_NAME);
        assert_eq!(
            said(true, Some(CELL), &[("\x07", "1.0")]).as_deref(),
            Some(UNNAMED),
            "a name of nothing but control bytes is no name"
        );
    }
}
