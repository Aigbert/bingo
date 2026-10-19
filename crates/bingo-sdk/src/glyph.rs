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
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn every_glyph_has_an_ascii_twin() {
        for glyph in Glyph::ALL {
            assert!(glyph.spell(GlyphSet::Ascii).is_ascii());
        }
        assert_eq!(Glyph::Heavy.spell(GlyphSet::Ascii), "=");
    }
}
