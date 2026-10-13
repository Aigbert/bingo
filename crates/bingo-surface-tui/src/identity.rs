//! M149's visible contracts, before their implementation.
use crate::{activity, opening, theme, welcome};
use crate::test_support::*;
use bingo_sdk::{Event, ItemStatus};
use ratatui::style::Color;

#[test]
fn ember_palette() {
    assert_eq!(theme::DARK.presence, Color::Rgb(0xf0, 0x88, 0x3e));
    assert_eq!(theme::DARK.glow, Color::Rgb(0xff, 0xc2, 0x78));
    assert_eq!(theme::LIGHT.presence, Color::Rgb(0xa8, 0x4d, 0x12));
    assert_eq!(theme::LIGHT.glow, Color::Rgb(0xe0, 0x7a, 0x2a));
}

#[test]
fn caret_glyphs() {
    assert_eq!(theme::spark(), "▍");
    theme::with(crate::painted::ascii(), || assert_eq!(theme::spark(), "|"));
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
        state.apply(&frame(2, Event::ItemStarted {
            item: assistant("itm_1", "words", ItemStatus::Running),
        }));
        assert_eq!(mark(&state, &ui, now).style, theme::glow());
        assert_eq!(mark(&state, &ui, later(now, 530)).style, theme::glow());
        ui.stop_asked = state.turn.as_ref().map(|turn| turn.id.clone());
        assert_eq!(mark(&state, &ui, now).style, theme::dim());
    });
}

#[test]
fn opening_wordmark_and_frames() {
    let rows = welcome::lines(&state(), 80, None);
    assert_eq!(rows[0].to_string(), "█      ▀");
    assert_eq!(rows[1].to_string(), "█▀▀▀▀█ █ █▀▀▀▀█ █▀▀▀▀█ █▀▀▀▀█");
    assert_eq!(rows[2].to_string(), "█▄▄▄▄█ █ █    █ █▄▄▄▄█ █▄▄▄▄█ ▍");
    assert_eq!(rows[3].to_string(), "                ▄▄▄▄▄█");
    assert_eq!(welcome::lines(&state(), 38, None)[0].to_string(), "bingo▍");
    for t in [0.0, 0.5, 1.2, 2.0, 2.4] {
        let frame = opening::frame(t, &rows);
        assert_eq!(frame.len(), rows.len());
        insta::assert_snapshot!(format!("wordmark_{t:.1}"),
            frame.iter().map(ToString::to_string).collect::<Vec<_>>().join("\n"));
    }
}

#[test]
fn code_language_shares_the_raised_band() {
    theme::with(crate::painted::truecolor(), || {
        let rows = crate::views::code::lines(Some("rust"), "fn main() {}\n// end", 40);
        assert_eq!(rows.len(), 2);
        assert!(rows[0].to_string().ends_with("rust"));
        assert_eq!(rows[0].style.bg, Some(theme::DARK.raised));
    });
}
