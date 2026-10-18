//! M149's visible contracts, before their implementation.
use crate::test_support::*;
use crate::{activity, theme, welcome};
use bingo_sdk::{Event, ItemStatus};
use ratatui::style::Color;

#[test]
fn ember_palette() {
    assert_eq!(theme::DARK.ember, Color::Rgb(0xf3, 0x90, 0x50));
    assert_eq!(theme::DARK.glow, Color::Rgb(0xff, 0x9b, 0x4a));
    assert_eq!(theme::LIGHT.ember, Color::Rgb(0xb0, 0x4c, 0x14));
    assert_eq!(theme::LIGHT.glow, Color::Rgb(0xef, 0x7a, 0x2c));
}

#[test]
fn caret_glyphs() {
    assert_eq!(theme::mark(), "▍");
    theme::with(crate::painted::ascii(), || assert_eq!(theme::mark(), "|"));
}

#[test]
fn caret_phase_styles() {
    let (mut ui, now) = mid_turn();
    let mut state = folded(vec![frame(1, started("trn_1"))]);
    let mark = |state: &bingo_sdk::SessionState, ui: &crate::ui::Ui, now| {
        activity::lines(&solo(state), ui, 80, now)[1].spans[0].clone()
    };
    theme::with(crate::painted::truecolor(), || {
        let on = mark(&state, &ui, now);
        let off = mark(&state, &ui, later(now, 530));
        assert_ne!(on.style, off.style, "waiting blinks at 530 ms");
        state.apply(&frame(
            2,
            Event::ItemStarted {
                item: assistant("itm_1", "words", ItemStatus::Running),
            },
        ));
        assert_eq!(mark(&state, &ui, now).style, theme::glow());
        assert_eq!(mark(&state, &ui, later(now, 530)).style, theme::glow());
        ui.stop_asked = state.turn.as_ref().map(|turn| turn.id.clone());
        assert_eq!(mark(&state, &ui, now).style, theme::dim());
    });
}

#[test]
fn opening_wordmark_and_frames() {
    let rows = welcome::lines(&state(), 80, None);
    assert_eq!(rows.len(), 1);
    assert!(rows[0].to_string().starts_with(" bingo▍ /tmp/project"));
    assert!(rows[0].to_string().contains("session 1"));
}

#[test]
fn code_language_shares_the_raised_band() {
    theme::with(crate::painted::truecolor(), || {
        let rows = crate::views::code::lines(Some("rust"), "fn main() {}\n// end", 40);
        assert_eq!(rows.len(), 2);
        assert!(rows[0].to_string().ends_with("rust"));
        assert_eq!(rows[0].style.bg, Some(theme::DARK.you));
    });
}

#[test]
fn embers_hold_the_band_and_yield_to_motion_off_and_the_foot() {
    let (mut ui, now) = mid_turn();
    let mut state = folded(vec![frame(1, started("trn_1"))]);
    state.apply(&frame(
        2,
        Event::ItemStarted {
            item: assistant("itm_1", &"a".repeat(1024), ItemStatus::Running),
        },
    ));
    assert_eq!(crate::embers::tokens(&state), 256);
    ui.embers.observe(0, &state, now.instant);
    let tree = solo(&state);
    let with = activity::lines(&tree, &ui, 80, now);
    assert_eq!(with[0].to_string(), " · · · ·");
    assert_eq!(
        with.len(),
        activity::lines(&tree, &ui, 80, still(now)).len()
    );
    assert!(
        activity::lines(&tree, &ui, 80, still(now))[0]
            .to_string()
            .is_empty()
    );
    assert!(ui.embers.row(&state, now, 80, true).to_string().is_empty());
    let drawn = draw_tree(80, 24, &tree, &mut ui, now);
    assert!(drawn.contains(" · · · ·"));
    assert!(
        activity::lines(&tree, &ui, 80, later(now, 600))[0]
            .to_string()
            .is_empty()
    );
    state.turn = None;
    ui.embers.observe(256, &state, now.instant);
    assert!(ui.embers.row(&state, now, 80, false).to_string().is_empty());
}
