//! Which terminals draw a `U=1` placeholder cell, by name and version.
//!
//! This is a list because it cannot be a question. The kitty graphics
//! protocol's own query (`a=q`) answers `OK` or an error and carries no
//! feature bits, and neither DA1 nor XTVERSION has a field for placeholders
//! (spec and replies read 2026-09-04). Asking by side effect is worse than
//! not asking: WezTerm and Konsole parse the `U` key, ignore it, and draw a
//! real placement at the cursor, so a probe would leave a picture stuck to
//! the screen on exactly the terminals it is trying to rule out.
//!
//! So a terminal draws pictures here only if it says `OK`, says how big a
//! cell is, *and* names itself as one of these four — or as the one
//! multiplexer that forwards the cells without saying which version it is.
//! Silence is not a yes.

use super::probe::{Named, Version};

/// The terminals known to draw placeholder cells, and the version each of
/// them learned to. Case is not part of a name.
///
/// Before adding a fifth, read its parser rather than its changelog: the two
/// that get this wrong both *store* the `U` key and never read it. The four
/// here, with what was read on 2026-09-04:
///
/// - **kitty 0.28.0** — the protocol's own "Unicode placeholders" section is
///   marked `versionadded: 0.28.0` (PR kovidgoyal/kitty#5664).
/// - **Ghostty 1.0.0** — ghostty-org/ghostty#2015, merged 2024-07-31, before
///   the 1.0 release.
/// - **iTerm2 3.5.6** — gnachman/iTerm2 commit 4fe5b21 (2024-08-21), first in
///   the 3.5.6 stable of 2024-11-02.
/// - **Rio 0.5.27** — raphamorim/rio#1893 (2026-08-24), fixing #1891.
///
/// One more draws them with no version to hold it to: [`FORWARDS`].
///
/// And the ones that answer `OK` and get the chip instead: WezTerm
/// (wezterm#986, open since 2021-07-28), Konsole (`Vt102Emulation.cpp` stores
/// `U` and never reads it; bugs.kde.org 523718), Warp (warp#6210, rejects the
/// placement silently under `q=2`), VS Code's xterm.js (xterm.js#5711).
const FLOORS: [(&str, Version); 4] = [
    ("kitty", [0, 28, 0]),
    ("ghostty", [1, 0, 0]),
    ("iterm2", [3, 5, 6]),
    ("rio", [0, 5, 27]),
];

/// The multiplexer that answers XTVERSION with a name and no version, and
/// forwards the placeholder cells all the same: herdr, whose panes are
/// libghostty terminals and answer `>|libghostty` and nothing more.
///
/// - **herdr 0.7.5** with `[experimental] kitty_graphics = true` (the default
///   from 0.9.0) — its `kitty_virtual_image_placements` decodes the picture,
///   sends it on to the terminal in front and places it row by row over the
///   `U+10EEEE` cells, which it hides. Verified 2026-10-16 in an isolated
///   herdr 0.7.5 in front of a Ghostty 1.3.1.
///
/// With the setting off, a 0.7.5 pane answers `OK` all the same but leaks the
/// cells as text. What tells the two apart is the cell: off, herdr gives the
/// pane a window of no pixels and answers no `CSI 16 t`, so there is no cell
/// size and no picture.
pub(super) const FORWARDS: &str = "libghostty";

/// Whether this terminal draws the placeholder cells a virtual placement is
/// made of. A name that is not on the list, and a version below its floor,
/// both answer no — as does a version that will not parse, since a terminal
/// that cannot say which one it is cannot be held to a floor. The one name
/// held to none is [`FORWARDS`], and only when it says no version at all.
pub fn draws_placeholders(terminal: &Named) -> bool {
    if terminal.name.eq_ignore_ascii_case(FORWARDS) {
        return terminal.version.is_empty();
    }
    FLOORS.iter().any(|(name, floor)| {
        terminal.name.eq_ignore_ascii_case(name)
            && terminal.number().is_some_and(|said| said >= *floor)
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn draws(name: &str, version: &str) -> bool {
        draws_placeholders(&Named {
            name: name.into(),
            version: version.into(),
        })
    }

    /// The four, each at its floor and each one release below it.
    #[test]
    fn the_four_draw_placeholders_from_the_version_that_learned_to() {
        for (name, at, below) in [
            ("kitty", "0.28.0", "0.27.9"),
            ("ghostty", "1.0.0", "0.9.9"),
            ("iTerm2", "3.5.6", "3.5.5"),
            ("Rio", "0.5.27", "0.5.26"),
        ] {
            assert!(draws(name, at), "{name} {at}");
            assert!(!draws(name, below), "{name} {below}");
        }
    }

    /// And each of them well past its floor, in the spelling its own
    /// XTVERSION reply uses.
    #[test]
    fn a_terminal_past_its_floor_still_draws() {
        assert!(draws("kitty", "0.46.2"));
        assert!(draws("ghostty", "1.3.1"));
        assert!(draws("iTerm2", "3.6.11"));
        assert!(draws("Rio", "0.6.0"));
        assert!(draws("KITTY", "0.46.2"), "a name is not case");
    }

    /// The four that answer `OK` and draw tofu. These are the whole reason
    /// the list exists.
    #[test]
    fn a_terminal_that_says_ok_and_draws_no_placeholder_is_not_on_the_list() {
        assert!(!draws("WezTerm", "20240203-110809-5046fc22"));
        assert!(!draws("Konsole", "26.08.0"));
        assert!(!draws("Warp", "v0.2026.06.10.08.11.stable_02"));
        assert!(!draws("xterm.js", "5.5.0"));
        assert!(!draws("foot", "1.28.0"), "nor one with no graphics at all");
    }

    /// A version nobody can compare is below every floor: silence is not a
    /// yes, and neither is noise.
    #[test]
    fn a_version_that_will_not_parse_is_below_every_floor() {
        for said in ["", "unknown", "v1.0.0", ".", "x.28.0"] {
            assert!(!draws("kitty", said), "{said:?}");
            assert!(!draws("ghostty", said), "{said:?}");
        }
    }

    /// herdr's pane names itself and no version, and is held to no floor —
    /// but only by exactly that name and only with no version: a libghostty
    /// that did say one is not the reply that was read.
    #[test]
    fn herdrs_bare_libghostty_draws_with_no_version_and_nothing_else_does() {
        assert!(draws("libghostty", ""));
        assert!(draws("LibGhostty", ""), "a name is not case");
        assert!(!draws("libghostty", "1.3.1"), "not the reply that was read");
        assert!(!draws("libghostty-vt", ""));
        assert!(!draws("herdr", ""));
    }
}
