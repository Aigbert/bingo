//! Completion sparks paint only empty cells above a visible prompt band.
use crate::{
    clock::{self, Now},
    theme,
};
use bingo_sdk::{ItemId, SessionState, TurnStatus};
use ratatui::{buffer::Buffer, layout::Rect, style::Color};

pub fn anchor(state: &SessionState) -> Option<&ItemId> {
    if let Some(shell) = super::ran::latest(state) {
        return matches!(shell.body, bingo_sdk::ItemBody::Shell { exit: Some(0), .. })
            .then_some(&shell.id);
    }
    let last = state.last_turn.as_ref()?;
    if state.turn.is_some() || last.status != TurnStatus::Completed {
        return Default::default();
    }
    state
        .transcript
        .items()
        .iter()
        .find(|item| item.turn.as_ref() == Some(&last.id) && super::band::owns(item, state))
        .map(|item| &item.id)
}

pub fn draw(
    state: &SessionState,
    now: Now,
    buffer: &mut Buffer,
    area: Rect,
    row: u16,
) -> crate::views::Drawing {
    if !now.motion || theme::glyphs::get("side").is_ascii() {
        return Default::default();
    }
    let end = super::ran::latest(state)
        .and_then(|item| item.completed_at)
        .or_else(|| state.last_turn.as_ref().map(|turn| turn.ended_at));
    let Some(end) = end else {
        return Default::default();
    };
    let k = now.wall.duration_since(end).as_millis() as f64 / 700.0;
    if !(0.0..1.0).contains(&k) {
        return Default::default();
    }
    let e = 1.0 - (1.0 - k).powi(3);
    for (dx, dy) in [(0.6, 1.15), (1.5, 1.4), (2.4, 0.95)] {
        let x = 0.5 + dx * 2.6 * e;
        let y = f64::from(row) + 0.45 - dy * 2.2 * e;
        dot(buffer, area, x, y, k);
    }
    crate::views::Drawing {
        lines: Vec::new(),
        next: Some(now.instant + clock::FRAME),
    }
}

fn dot(buffer: &mut Buffer, area: Rect, x: f64, y: f64, age: f64) {
    if x < 0.0 || y < 0.0 || x >= f64::from(area.width) || y >= f64::from(area.height) {
        return;
    }
    let cell = &mut buffer[(area.x + x.floor() as u16, area.y + y.floor() as u16)];
    if cell.symbol() != " " {
        return;
    }
    let bits = [[1, 2, 4, 64], [8, 16, 32, 128]];
    let bit = bits[(x.fract() * 2.0) as usize][(y.fract() * 4.0) as usize];
    let Some(glyph) = char::from_u32(0x2800 + bit) else {
        return;
    };
    let ground = if cell.bg == Color::Reset {
        theme::palette().bg()
    } else {
        cell.bg
    };
    cell.set_char(glyph)
        .set_style(theme::literal(theme::ink::mix(
            theme::palette().glow(),
            ground,
            age.powf(1.3),
        )));
}
