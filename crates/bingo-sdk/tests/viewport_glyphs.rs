use bingo_sdk::{glyph::GlyphSet, render::Viewport};
use serde_json::Value;

#[test]
fn viewports_default_to_unicode_and_round_trip_ascii() {
    let cases: Vec<Value> =
        serde_json::from_str(include_str!("fixtures/viewport_glyphs.json")).unwrap();
    for (value, glyphs) in cases.into_iter().zip([GlyphSet::Unicode, GlyphSet::Ascii]) {
        let viewport: Viewport = serde_json::from_value(value.clone()).unwrap();
        assert_eq!(viewport.glyphs, glyphs);
        assert_eq!(serde_json::to_value(viewport).unwrap(), value);
    }
}
