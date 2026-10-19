//! The shared Ember glyph spellings, independent of a rendering surface.
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum GlyphSet {
    #[default]
    Unicode,
    Ascii,
}
impl GlyphSet {
    pub fn is_unicode(&self) -> bool {
        *self == Self::Unicode
    }
}
macro_rules! glyphs {
    ($(($variant:ident, $name:literal, $unicode:literal, $ascii:literal)),+ $(,)?) => {
        #[derive(Clone, Copy, Debug, PartialEq, Eq)]
        pub enum Glyph { $($variant),+ }
        impl Glyph {
            pub const ALL: &'static [Self] = &[$(Self::$variant),+];
            pub const fn spell(self, set: GlyphSet) -> &'static str {
                match (self, set) {
                    $((Self::$variant, GlyphSet::Unicode) => $unicode,
                      (Self::$variant, GlyphSet::Ascii) => $ascii),+
                }
            }
            pub fn named(name: &str) -> Option<Self> {
                match name {
                    $($name => Some(Self::$variant),)+
                    _ => None,
                }
            }
        }
    };
}
glyphs! {
    (Lamp, "lamp", "●", "*"),
    (Bullet, "bullet", "•", "-"),
    (Sep, "sep", " · ", " - "),
    (Ok, "ok", "✓", "+"),
    (Still, "still", "◌", "o"),
    (Edge, "edge", "▔", "-"),
    (Base, "base", "▁", "_"),
    (Thin, "thin", "─", "-"),
    (Vbar, "vbar", "│", "|"),
    (Quote, "quote", "▎", "|"),
    (Caret, "caret", "▍", "|"),
    (Ell, "ell", "…", "..."),
    (Arrow, "arrow", "→", "->"),
    (Up, "up", "↑", "^"),
    (Down, "down", "↓", "v"),
    (Minus, "minus", "−", "-"),
    (Times, "times", "×", "x"),
    (Dot, "dot", "·", "."),
    (Ret, "ret", "⏎", "enter"),
    (Pin, "pin", "◆", "*"),
    (Side, "side", "▍", "|"),
    (Think, "think", "≈", ":"),
    (Spin, "spin", "⠋⠙⠹⠸⠼⠴⠦⠧⠇⠏", "-\\|/"),
    (Heavy, "heavy", "━", "="),
}
/// Key glyphs and the words an ASCII terminal spells them with, longest first.
pub const KEY_WORDS: &[(&str, &str)] = &[
    ("pgup pgdn", "pgup/pgdn"),
    ("⇧⏎", "shift+enter"),
    ("⇧tab", "shift+tab"),
    ("⏎", "enter"),
    ("↑↓", "up/down"),
    ("←→", "left/right"),
    ("↑", "up"),
    ("↓", "down"),
    ("⌫", "backspace"),
];

/// Key text as the glyph set spells it: glyphs stay glyphs in unicode and
/// become words in ASCII.
pub fn key_text(text: &str, set: GlyphSet) -> String {
    if set.is_unicode() {
        return text.to_string();
    }
    KEY_WORDS
        .iter()
        .fold(text.to_string(), |text, (from, to)| text.replace(from, to))
}

/// One chord as it is bound (`shift+tab`), as the glyph set draws it.
pub fn chord(bound: &str, set: GlyphSet) -> String {
    let glyph = KEY_WORDS
        .iter()
        .find(|(_, words)| *words == bound)
        .map_or(bound, |(glyph, _)| glyph);
    key_text(glyph, set)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn keys_are_glyphs_in_unicode_and_words_in_ascii() {
        assert!(KEY_WORDS.iter().all(|(_, words)| words.is_ascii()));
        assert_eq!(chord("shift+tab", GlyphSet::Unicode), "⇧tab");
        assert_eq!(chord("shift+tab", GlyphSet::Ascii), "shift+tab");
        assert_eq!(chord("alt+m", GlyphSet::Unicode), "alt+m");
        assert_eq!(
            key_text("⇧⏎ ⇧tab ⏎ ↑↓ ←→ ↑ ↓ ⌫", GlyphSet::Ascii),
            "shift+enter shift+tab enter up/down left/right up down backspace"
        );
    }
    #[test]
    fn every_glyph_has_an_ascii_twin() {
        for glyph in Glyph::ALL {
            assert!(glyph.spell(GlyphSet::Ascii).is_ascii());
        }
        assert_eq!(Glyph::Heavy.spell(GlyphSet::Ascii), "=");
    }
}
