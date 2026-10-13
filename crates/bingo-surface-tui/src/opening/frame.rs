//! Column typing, two blinks, three embers, then the resting wordmark.
use super::{
    END,
    cells::{self, Cell},
};
use crate::theme;
use ratatui::text::Line;

pub fn frame(t: f32, rows: &[Line<'static>]) -> Vec<Line<'static>> {
    if t >= END {
        return rows.to_vec();
    }
    let compact = rows
        .first()
        .is_some_and(|row| row.to_string().starts_with("bingo"));
    let mark_rows = if compact { 1 } else { 4 };
    let caret_row = if compact { 0 } else { 2 };
    let end = rows
        .get(caret_row)
        .map_or(0, |row| row.width().saturating_sub(1));
    rows.iter()
        .enumerate()
        .map(|(y, row)| {
            let mut x = 0;
            let mut cells = cells::of(row);
            if y == 0 && !compact {
                cells.resize_with(end + 1, || Cell {
                    glyph: " ".into(),
                    style: theme::text(),
                });
            }
            let drawn = cells
                .into_iter()
                .map(|cell| {
                    let column = x;
                    x += cell.width();
                    if y < mark_rows {
                        word(t, y, column, cell, (caret_row, end))
                    } else {
                        beam(t, column, x.max(row.width()), cell)
                    }
                })
                .collect();
            cells::line(drawn)
        })
        .collect()
}

fn word(t: f32, y: usize, x: usize, cell: Cell, (caret_row, end): (usize, usize)) -> Cell {
    let head = (t.max(0.0) / 0.016) as usize;
    if x > head {
        return cell.blank();
    }
    if x == head.min(end) && y == caret_row {
        let hidden = (0.5..1.56).contains(&t) && ((t - 0.5) / 0.265) as usize % 2 == 1;
        return Cell {
            glyph: if hidden { " " } else { theme::mark() }.into(),
            style: theme::glow(),
        };
    }
    if caret_row > 0
        && y == 0
        && [end.saturating_sub(4), end.saturating_sub(2), end].contains(&x)
        && (1.56..2.16).contains(&t)
    {
        return Cell {
            glyph: "·".into(),
            style: theme::comet((t - 1.56) / 0.6),
        };
    }
    cell
}

fn beam(t: f32, x: usize, width: usize, cell: Cell) -> Cell {
    let head = ((t - 0.5).max(0.0) / 1.5 * width as f32) as usize;
    if !crate::clock::swept((t - 0.5) / 1.5, x, width) || t < 0.5 {
        return cell.blank();
    }
    if x + 2 >= head && t < 2.0 && !cell.blank_already() {
        Cell {
            style: theme::glow(),
            ..cell
        }
    } else {
        cell
    }
}
