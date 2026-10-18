//! M149's visible contracts, before their implementation.
use crate::test_support::*;
use crate::theme;
use ratatui::style::Color;

#[test]
fn ember_palette() {
    assert_eq!(theme::DARK.ember(), Color::Rgb(0xf3, 0x90, 0x50));
    assert_eq!(theme::DARK.glow(), Color::Rgb(0xff, 0x9b, 0x4a));
    assert_eq!(theme::LIGHT.ember(), Color::Rgb(0xb0, 0x4c, 0x14));
    assert_eq!(theme::LIGHT.glow(), Color::Rgb(0xef, 0x7a, 0x2c));
}

#[test]
fn caret_glyphs() {
    assert_eq!(theme::mark(), "▍");
    theme::with(crate::painted::ascii(), || assert_eq!(theme::mark(), "|"));
}

#[test]
fn opening_props_carry_the_session_identity() {
    let state = state();
    let folds = Default::default();
    let pictures = Default::default();
    let linked = Default::default();
    let rows = crate::transcript::Rows::of(&state, 80, &folds, &pictures, &linked, scene().1);
    let bingo_sdk::component::Props::Welcome(props) = crate::component::welcome::props(&rows)
    else {
        panic!("welcome");
    };
    assert_eq!(props.session, state.summary);
}

#[test]
fn code_language_shares_the_raised_band() {
    theme::with(crate::painted::truecolor(), || {
        let rows = crate::views::code::lines(Some("rust"), "fn main() {}\n// end", 40);
        assert_eq!(rows.len(), 2);
        assert!(rows[0].to_string().ends_with("rust"));
        assert_eq!(rows[0].style.bg, Some(theme::DARK.you()));
    });
}
