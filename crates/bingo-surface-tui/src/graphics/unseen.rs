//! Why a herdr pane that speaks the protocol still gets the chip.
//!
//! A terminal that draws no pictures has, as a rule, nothing a person could
//! change, and is told nothing. herdr is the exception: its pane answers the
//! kitty query `OK` and names itself whether or not it forwards pictures,
//! and the one thing that tells the two apart — no cell — has a setting that
//! turns it around. That setting is the line this module says.

use super::placeholders::{FORWARDS, draws_placeholders};
use super::{Heard, Transport};

/// herdr with pictures off: its pane says `OK` and `libghostty`, and gives
/// the pane a window of no pixels and no `CSI 16 t` answer. herdr 0.9 turned
/// the setting on by default.
pub const HERDR_OFF: &str = "herdr: pictures need 0.9, or `[experimental] kitty_graphics = true`";

/// The line there is to say when a herdr pane said `OK` and gave no cell to
/// draw into, and nothing otherwise.
///
/// Only bare: under a tmux the cell is not read off the window (that is
/// tmux's guess at the pixels), so a herdr behind tmux has no cell whether
/// its setting is on or off, and this line would be wrong half the time.
pub fn unseen(heard: &Heard) -> Option<&'static str> {
    let probe = &heard.probe;
    let herdr = probe
        .terminals
        .iter()
        .any(|named| named.name.eq_ignore_ascii_case(FORWARDS) && draws_placeholders(named));
    (heard.transport == Transport::Bare && probe.kitty && probe.cell.is_none() && herdr)
        .then_some(HERDR_OFF)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::graphics::Cell;
    use crate::graphics::Passthrough;
    use crate::graphics::probe::{Named, Probe};

    const CELL: Cell = Cell {
        width: 10,
        height: 20,
    };

    fn said(
        transport: Transport,
        kitty: bool,
        cell: Option<Cell>,
        names: &[(&str, &str)],
    ) -> Option<&'static str> {
        unseen(&Heard {
            probe: Probe {
                kitty,
                cell,
                terminals: names
                    .iter()
                    .map(|(name, version)| Named {
                        name: (*name).into(),
                        version: (*version).into(),
                    })
                    .collect(),
            },
            transport,
            passthrough: Passthrough::Unknown,
        })
    }

    const HERDR: (&str, &str) = ("libghostty", "");

    /// herdr with pictures off answers just as with them on, bar the cell:
    /// the line names the setting. With a cell there is nothing to say.
    #[test]
    fn a_bare_herdr_pane_with_no_cell_is_told_the_setting() {
        assert_eq!(said(Transport::Bare, true, None, &[HERDR]), Some(HERDR_OFF));
        assert_eq!(said(Transport::Bare, true, Some(CELL), &[HERDR]), None);
        assert_eq!(said(Transport::Bare, false, None, &[HERDR]), None);
    }

    /// Under tmux the window is not read, so herdr behind it has no cell
    /// with its setting on too: the line would be wrong, and is not said.
    #[test]
    fn a_herdr_behind_tmux_is_told_nothing() {
        assert_eq!(
            said(Transport::Tmux, true, None, &[("tmux", "3.6b"), HERDR]),
            None
        );
    }

    /// A terminal there is nothing to do about says nothing: one off the
    /// list, one below its floor, one with no name, one with no cell.
    #[test]
    fn every_other_terminal_is_told_nothing() {
        for names in [
            &[("WezTerm", "20240203-110809-5046fc22")][..],
            &[("kitty", "0.27.9")],
            &[("libghostty", "1.3.1")],
            &[],
        ] {
            assert_eq!(said(Transport::Bare, true, Some(CELL), names), None);
            assert_eq!(said(Transport::Bare, true, None, names), None);
        }
        assert_eq!(
            said(Transport::Bare, true, None, &[("ghostty", "1.3.1")]),
            None
        );
    }
}
