//! The prototype's seven embers, sampled on the frame clock.
use crate::{
    clock::{self, Now},
    theme,
};
use ratatui::{buffer::Buffer, layout::Rect};

pub(super) fn draw(buffer: &mut Buffer, area: Rect, row: u16, at: i64, now: Now) {
    if !now.motion || theme::glyphs::get("caret").is_ascii() {
        return;
    }
    let age = now.wall.as_millisecond().saturating_sub(at) as f64;
    let k = clock::prog(now, at, 900);
    let columns = ((1.0 - (1.0 - k).powi(3)) * 29.0).round();
    let x = (usize::from(area.width).saturating_sub(52) / 2).max(3) as f64 + columns + 1.0;
    for j in 0..7 {
        if age < j as f64 * 2800.0 / 7.0 {
            continue;
        }
        let k = (age / 2800.0 + j as f64 / 7.0) % 1.0;
        let wobble = ((k * 2.3 + j as f64 * 1.9) * std::f64::consts::PI).sin() * 0.55;
        dot(
            buffer,
            area,
            x + 0.35 + wobble + ((j * 5) % 3 - 1) as f64 * 0.3,
            f64::from(row) + 0.15 - k * 4.4,
            k,
        );
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
    if let Some(glyph) = char::from_u32(0x2800 + bit) {
        cell.set_char(glyph)
            .set_style(theme::literal(theme::ink::mix(
                theme::palette().glow(),
                theme::palette().bg(),
                age.powf(1.25),
            )));
    }
}

pub(super) fn dissolve(buffer: &mut Buffer, area: Rect, at: i64, now: Now) {
    let k = clock::prog(now, at, 520);
    if k <= 0.0 {
        return;
    }
    for y in 0..area.height {
        for x in 0..area.width {
            let cell = &mut buffer[(area.x + x, area.y + y)];
            if cell.symbol() == " " || cell.symbol().is_empty() {
                continue;
            }
            let r = ((usize::from(x) * 73 + usize::from(y) * 151) % 97) as f64 / 97.0;
            if k * 1.4 > r {
                let bit = [[1, 2, 4, 64], [8, 16, 32, 128]][usize::from(x & 1)]
                    [3 - (k * 5.0).floor().min(3.0) as usize];
                let glyph = if k < 0.55 + r * 0.3 {
                    char::from_u32(0x2800 + bit).unwrap_or(' ')
                } else {
                    ' '
                };
                let ink = if cell.fg == ratatui::style::Color::Reset {
                    theme::palette().fg()
                } else {
                    cell.fg
                };
                cell.set_char(glyph)
                    .set_style(theme::literal(theme::ink::mix(
                        ink,
                        theme::palette().bg(),
                        (k * 1.6).clamp(0.0, 1.0),
                    )));
            }
        }
    }
}
