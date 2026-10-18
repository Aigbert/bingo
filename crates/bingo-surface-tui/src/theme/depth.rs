//! Colour depth conversion, leaving the terminal-owned first sixteen slots alone.
use ratatui::style::{Color, Style};
/// The fixed xterm cube and grey ramp; indices 0–15 belong to the terminal.
pub(super) fn nearest(color: Color) -> Color {
    let Color::Rgb(red, green, blue) = color else {
        return color;
    };
    let distance = |index: u8| {
        let (r, g, b) = indexed_rgb(index);
        let square = |a: u8, b: u8| (i32::from(a) - i32::from(b)).pow(2);
        square(red, r) + square(green, g) + square(blue, b)
    };
    Color::Indexed(
        (16..=255)
            .min_by_key(|index| distance(*index))
            .unwrap_or(16),
    )
}

fn indexed_rgb(index: u8) -> (u8, u8, u8) {
    if index >= 232 {
        let v = 8 + (index - 232) * 10;
        return (v, v, v);
    }
    let index = index.saturating_sub(16);
    let cube = |v: u8| if v == 0 { 0 } else { 55 + v * 40 };
    (cube(index / 36), cube(index / 6 % 6), cube(index % 6))
}

pub(super) fn foreground(name: &str, color: Color) -> ratatui::style::Style {
    use super::Colors;
    use ratatui::style::{Modifier, Style};
    let quiet = matches!(name, "muted" | "ash" | "dim" | "faint");
    match super::current().colors {
        Colors::Plain if quiet => Style::new().add_modifier(Modifier::DIM),
        Colors::Plain => Style::new(),
        Colors::Ansi if matches!(name, "dim") => Style::new().add_modifier(Modifier::DIM),
        Colors::Ansi => Style::new().fg(ansi(name)),
        Colors::True(_) => Style::new().fg(color),
        Colors::Indexed(_) => Style::new().fg(nearest(color)),
    }
}

fn ansi(name: &str) -> Color {
    match name {
        "muted" | "ash" | "dim" | "faint" => Color::DarkGray,
        "glow" | "amberGlow" => Color::LightYellow,
        "ember" | "presence" | "coal" | "amber" | "code" | "num" => Color::Yellow,
        "good" | "str" => Color::Green,
        "bad" => Color::Red,
        "plan" | "fn" => Color::Blue,
        "auto" | "kw" => Color::Magenta,
        "ty" => Color::Cyan,
        _ => Color::Reset,
    }
}

pub(super) fn ground(name: &str, color: Color) -> ratatui::style::Style {
    use super::Colors;
    use ratatui::style::{Modifier, Style};
    match super::current().colors {
        Colors::True(_) => Style::new().bg(color),
        Colors::Indexed(_) => Style::new().bg(nearest(color)),
        _ if matches!(name, "pick" | "pickBg") => Style::new().add_modifier(Modifier::REVERSED),
        _ => Style::new(),
    }
}

pub(super) fn literal(color: Color) -> Style {
    use super::Colors;
    match super::current().colors {
        Colors::Plain => Style::new(),
        Colors::Indexed(_) => Style::new().fg(match color {
            Color::Black => Color::Indexed(0),
            Color::Red => Color::Indexed(1),
            Color::Green => Color::Indexed(2),
            Color::Yellow => Color::Indexed(3),
            Color::Blue => Color::Indexed(4),
            Color::Magenta => Color::Indexed(5),
            Color::Cyan => Color::Indexed(6),
            Color::Gray => Color::Indexed(7),
            Color::DarkGray => Color::Indexed(8),
            Color::LightRed => Color::Indexed(9),
            Color::LightGreen => Color::Indexed(10),
            Color::LightYellow => Color::Indexed(11),
            Color::LightBlue => Color::Indexed(12),
            Color::LightMagenta => Color::Indexed(13),
            Color::LightCyan => Color::Indexed(14),
            Color::White => Color::Indexed(15),
            other => nearest(other),
        }),
        _ => Style::new().fg(color),
    }
}
