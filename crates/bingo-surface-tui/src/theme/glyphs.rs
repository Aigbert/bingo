//! Glyph vocabulary and the legacy screen spellings.
use ratatui::symbols::border;
const SPIN_U: &str = "⠋⠙⠹⠸⠼⠴⠦⠧⠇⠏";
const SPIN_A: &str = "-\\|/";

/// The forms of `docs/design/tui.md` §4, and the ASCII spellings of them.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Glyphs {
    /// What the model says and does, and what a session is.
    pub bullet: &'static str,
    /// What came back.
    pub connector: &'static str,
    /// What you said.
    pub user: &'static str,
    /// The row a card or a dropdown is talking to.
    pub cursor: &'static str,
    /// Where typing lands, in a row the terminal's own cursor is not parked
    /// on — a query line.
    pub caret: &'static str,
    /// bingo speaking and thinking, respectively.
    pub mark: [&'static str; 2],
    /// A box a person ticks: `[0]` open, `[1]` done. One field because it is
    /// one fact in two states, as the marks are one voice in two states.
    pub todo: [&'static str; 2],
    /// A task on the session's list, by where it stands: `[0]` still to do,
    /// `[1]` being done, `[2]` done — Claude Code's own three (M74), which
    /// are not the box a person ticks: a task is the model's to move on.
    pub task: [&'static str; 3],
    /// A tick with no box drawn round it: what a form's `Submit` tab wears and
    /// what stands inside a multi-select's own brackets.
    pub tick: &'static str,
    /// The permission mode in the footer.
    pub mode: &'static str,
    pub chrome: ChromeGlyphs,
    /// A tree node: `[0]` with siblings under it, `[1]` the last one. One
    /// field because it is one fact in two states, as the todo box is.
    pub tree: [&'static str; 2],
    /// What is left where something was folded away or cut short.
    pub ellipsis: &'static str,
    /// What opens an item of a list.
    pub point: &'static str,
    pub border: border::Set<'static>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ChromeGlyphs {
    pub pause: &'static str,
    /// A step of a turn: `[0]` done, `[1]` asks a person, `[2]` waits — what
    /// a running step stands still as where nothing moves — `[3]` failed.
    pub steps: [&'static str; 4],
    /// A running step turns through these, one per `spin`, on the wall clock
    /// so every running row is on the same frame.
    pub spinner: &'static str,
    pub spin: std::time::Duration,
    pub stop: &'static str,
    pub receipts: [&'static str; 4],
    pub context: [&'static str; 2],
}

pub const UNICODE: Glyphs = Glyphs {
    bullet: "▍",
    connector: "⎿",
    user: "›",
    cursor: "❯",
    caret: "▌",
    mark: ["▍", "▏"],
    todo: ["☐", "☒"],
    task: ["◻", "◼", "✔"],
    tick: "✔",
    mode: "⏵⏵",
    chrome: ChromeGlyphs {
        pause: "⏸",
        // `◆` is East-Asian ambiguous: a CJK-width terminal draws it in two
        // cells, and the row after it shifts by one there.
        steps: ["✓", "◆", "◌", "✗"],
        spinner: SPIN_U,
        spin: std::time::Duration::from_millis(80),
        stop: "■",
        receipts: ["⇄", "✎", "⟲", "⇣"],
        context: ["▰", "▱"],
    },
    tree: ["├", "└"],
    ellipsis: "…",
    point: "•",
    border: border::ROUNDED,
};

/// `BINGO_ASCII=1`: the six characters of §7, each doing the job its shape
/// suggests — `>` says you, `*` is a bullet, `+` sparkles and turns a corner,
/// `x` crosses a box off and ticks one, `-` connects and rules, `|` stands a
/// wall up and stands where typing lands.
///
/// A skill spends no seventh character: the row says `Skill(guide)` in words,
/// so the glyph is what a glance finds and never the only place the fact is.
pub const ASCII: Glyphs = Glyphs {
    bullet: "*",
    connector: "-",
    user: ">",
    cursor: ">",
    caret: "|",
    mark: ["|", "|"],
    todo: ["-", "x"],
    // Still to do, being done — the bullet, because it is the one the model
    // is on — and done.
    task: ["-", "*", "x"],
    tick: "x",
    mode: ">>",
    chrome: ChromeGlyphs {
        pause: "||",
        steps: ["+", "*", "o", "!"],
        spinner: SPIN_A,
        spin: std::time::Duration::from_millis(120),
        stop: "x",
        receipts: ["=", "/", "<", "v"],
        context: ["#", "-"],
    },
    tree: ["+", "+"],
    ellipsis: "...",
    point: "-",
    border: border::Set {
        top_left: "+",
        top_right: "+",
        bottom_left: "+",
        bottom_right: "+",
        vertical_left: "|",
        vertical_right: "|",
        horizontal_top: "-",
        horizontal_bottom: "-",
    },
};

pub fn set() -> bingo_sdk::glyph::GlyphSet {
    if super::glyphs() == &ASCII {
        bingo_sdk::glyph::GlyphSet::Ascii
    } else {
        bingo_sdk::glyph::GlyphSet::Unicode
    }
}
pub fn get(name: &str) -> &'static str {
    bingo_sdk::glyph::Glyph::named(name).map_or("", |glyph| glyph.spell(set()))
}

pub fn chord(ascii: &str) -> String {
    bingo_sdk::glyph::chord(ascii, set())
}

pub fn key_text(text: &str) -> String {
    bingo_sdk::glyph::key_text(text, set())
}

#[cfg(test)]
mod tests {
    use super::*;
    use unicode_width::{UnicodeWidthChar, UnicodeWidthStr};

    #[test]
    fn prototype_glyphs_have_their_documented_cell_widths() {
        use bingo_sdk::glyph::{Glyph, GlyphSet};
        for set in [GlyphSet::Unicode, GlyphSet::Ascii] {
            for &name in Glyph::ALL {
                let glyph = name.spell(set);
                if name == Glyph::Spin {
                    for frame in glyph.chars() {
                        assert_eq!(frame.width(), Some(1));
                    }
                } else if !(matches!(name, Glyph::Sep | Glyph::Ell | Glyph::Ret)
                    || set == GlyphSet::Ascii && name == Glyph::Arrow)
                {
                    assert_eq!(glyph.width(), 1, "{name:?}: {glyph}");
                }
            }
        }
        assert_eq!(Glyph::Arrow.spell(GlyphSet::Ascii), "->");
        super::super::with(crate::painted::ascii(), || {
            assert_eq!(
                key_text("⇧⏎ ⇧tab ⏎ ↑↓ ←→ ↑ ↓ ⌫"),
                "shift+enter shift+tab enter up/down left/right up down backspace"
            );
        });
    }
}
