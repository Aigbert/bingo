use super::{Band, Capture, Row, parse};
use crate::theme;
use ratatui::{buffer::Buffer, layout::Rect, style::Color};

pub(super) fn compare(
    capture: &Capture,
    band: Band,
    buffer: &Buffer,
    area: Rect,
) -> Result<(), String> {
    let expected = capture.area(band);
    if (expected.width, expected.height) != (area.width, area.height) {
        return Err(format!(
            "{} {band:?}: expected {}×{}, rendered {}×{}",
            capture.entry.file, expected.width, expected.height, area.width, area.height
        ));
    }
    for y in 0..expected.height {
        let row = &capture.rows[usize::from(expected.y + y)];
        for x in 0..expected.width {
            if let Some(reason) = mismatch(row, x, buffer, area.x + x, area.y + y) {
                return Err(report(capture, band, buffer, area, (x, y), &reason));
            }
        }
    }
    Ok(())
}

fn mismatch(row: &Row, x: u16, buffer: &Buffer, bx: u16, by: u16) -> Option<String> {
    let symbol = &row.cells[usize::from(x)];
    let cell = &buffer[(bx, by)];
    if !symbol.is_empty() && !same_symbol(symbol, cell.symbol()) {
        return Some(format!(
            "character: expected {symbol:?}, rendered {:?}",
            cell.symbol()
        ));
    }
    // Ratatui clears the continuation cell of a wide glyph; its foreground is the leader's.
    let fg = if symbol.is_empty() && x > 0 {
        buffer[(bx - 1, by)].fg
    } else {
        cell.fg
    };
    for (layer, runs, color, reset) in [("fg", &row.fg, fg, "fg"), ("bg", &row.bg, cell.bg, "bg")] {
        let Some(token) = parse::token(runs, x) else {
            continue;
        };
        let name = token.split('+').next().unwrap();
        if name.starts_with('#') || matches!(name, "kw" | "str" | "num" | "fn" | "ty") {
            continue;
        }
        let expected = theme::LIGHT
            .named(name)
            .unwrap_or_else(|| panic!("unknown capture ink {name}"));
        let actual = if color == Color::Reset {
            theme::LIGHT.named(reset).unwrap()
        } else {
            color
        };
        if layer == "fg" && !symbol.is_empty() {
            let mut expected = ratatui::style::Modifier::empty();
            for part in token.split('+').skip(1) {
                expected |= match part {
                    "b" => ratatui::style::Modifier::BOLD,
                    "i" => ratatui::style::Modifier::ITALIC,
                    "u" => ratatui::style::Modifier::UNDERLINED,
                    _ => panic!("unknown capture modifier {part}"),
                };
            }
            if cell.modifier != expected {
                return Some(format!(
                    "modifiers: expected {expected:?}, rendered {:?}",
                    cell.modifier
                ));
            }
        }
        if actual != expected {
            return Some(format!(
                "{layer}: expected {name} {expected:?}, rendered {actual:?}"
            ));
        }
    }
    None
}

fn same_symbol(expected: &str, actual: &str) -> bool {
    expected == actual
        || ["⠋⠙⠹⠸⠼⠴⠦⠧⠇⠏", "-\\|/"].iter().any(|frames| {
            expected.chars().count() == 1
                && actual.chars().count() == 1
                && frames.contains(expected)
                && frames.contains(actual)
        })
}

fn report(
    capture: &Capture,
    band: Band,
    buffer: &Buffer,
    area: Rect,
    at: (u16, u16),
    reason: &str,
) -> String {
    let expected = capture.area(band);
    let mut out = format!(
        "{} {band:?}: first difference at cell ({}, {}): {reason}\n     capture{} | rendered\n",
        capture.entry.file,
        at.0,
        at.1,
        " ".repeat(usize::from(area.width.saturating_sub(7)))
    );
    for y in 0..area.height {
        let row = &capture.rows[usize::from(expected.y + y)];
        let actual = buffer_row(buffer, area, y);
        out += &format!("{y:03}  {} | {actual}\n", row.cells.concat());
        if y == at.1 {
            out += &format!(
                "     {}^{} | {}^\n",
                " ".repeat(usize::from(at.0)),
                " ".repeat(usize::from(area.width - at.0 - 1)),
                " ".repeat(usize::from(at.0))
            );
            out += &format!("     capture ink: bg {:?}; fg {:?}\n", row.bg, row.fg);
            out += &format!("     rendered ink: {}\n", ink_runs(buffer, area, y));
        }
    }
    out
}

fn buffer_row(buffer: &Buffer, area: Rect, y: u16) -> String {
    let mut text = String::new();
    let mut skip = 0;
    for x in area.x..area.right() {
        if skip > 0 {
            skip -= 1;
            continue;
        }
        let symbol = buffer[(x, area.y + y)].symbol();
        skip = unicode_width::UnicodeWidthStr::width(symbol).saturating_sub(1);
        text += symbol;
    }
    text
}

fn ink_runs(buffer: &Buffer, area: Rect, y: u16) -> String {
    let mut runs: Vec<(u16, u16, ratatui::style::Style)> = Vec::new();
    for x in 0..area.width {
        let style = buffer[(area.x + x, area.y + y)].style();
        if let Some(last) = runs.last_mut()
            && last.2 == style
        {
            last.1 = x;
        } else {
            runs.push((x, x, style));
        }
    }
    runs.iter()
        .map(|(a, b, style)| format!("{a}-{b} {style:?}"))
        .collect::<Vec<_>>()
        .join(" · ")
}
