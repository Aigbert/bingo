//! A host-only cell mask travels through the same layout as the drawing.
use super::Marks;
use bingo_sdk::View;
use ratatui::{layout::Rect, text::Line};

const FIRST: u32 = 0x100000;

pub(super) fn mask(view: &View, lines: Vec<Line<'static>>, marks: &Marks) -> Vec<Line<'static>> {
    let Some(expected) = &marks.sealed else {
        return lines;
    };
    if matches!(view, View::Box { .. } | View::Panel { .. }) {
        return lines;
    }
    if !matches!(view, View::Engine { .. }) {
        return lines
            .into_iter()
            .map(|line| Line::from(" ".repeat(line.width())))
            .collect();
    }
    let widths: Vec<_> = lines.iter().map(|line| line.width().max(1)).collect();
    *expected.borrow_mut() = widths.clone();
    widths
        .into_iter()
        .enumerate()
        .map(|(row, width)| {
            let symbol = char::from_u32(FIRST + row as u32).unwrap_or(' ');
            Line::from(symbol.to_string().repeat(width))
        })
        .collect()
}

/// Every engine row must survive once, in order, with every cell inside the card.
pub fn whole(
    lines: &[Line<'_>],
    expected: &[usize],
    width: usize,
    rows: usize,
) -> Option<Vec<Rect>> {
    if expected.is_empty() || lines.len() > rows || lines.iter().any(|line| line.width() > width) {
        return None;
    }
    let mut found = vec![Vec::new(); expected.len()];
    for (y, line) in lines.iter().enumerate() {
        let mut x = 0;
        for ch in line.to_string().chars() {
            if let Some(row) = (ch as u32).checked_sub(FIRST) {
                found.get_mut(row as usize)?.push((x, y));
            }
            x += unicode_width::UnicodeWidthChar::width(ch).unwrap_or(0);
        }
    }
    let mut areas = Vec::new();
    for (cells, width) in found.iter().zip(expected) {
        let &(x, y) = cells.first()?;
        if cells.len() != *width
            || cells
                .iter()
                .enumerate()
                .any(|(i, cell)| *cell != (x + i, y))
            || areas
                .last()
                .is_some_and(|last: &Rect| usize::from(last.y) + 1 != y)
        {
            return None;
        }
        areas.push(Rect::new(
            u16::try_from(x).ok()?,
            u16::try_from(y).ok()?,
            u16::try_from(*width).ok()?,
            1,
        ));
    }
    Some(areas)
}

#[cfg(test)]
mod tests;
