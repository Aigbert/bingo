//! Tables keep numeric columns aligned, quiet headers and three-cell gutters.
use ratatui::text::{Line, Span};

use super::{MISSING, numeric};

struct Column {
    width: usize,
    numeric: bool,
}

/// Tables keep three cells between columns and a single rule below the header.
pub fn lines(headers: &[String], rows: &[Vec<String>], width: usize) -> Vec<Line<'static>> {
    let heading: Vec<_> = (0..headers.len())
        .map(|at| content(cell(headers, at), true, width))
        .collect();
    let drawn: Vec<Vec<_>> = rows
        .iter()
        .map(|row| {
            (0..headers.len())
                .map(|at| content(cell(row, at), false, width))
                .collect()
        })
        .collect();
    let columns: Vec<_> = headers
        .iter()
        .enumerate()
        .map(|(at, _)| Column {
            width: std::iter::once(&heading[at])
                .chain(drawn.iter().map(|row| &row[at]))
                .map(Line::width)
                .max()
                .unwrap_or(0),
            numeric: rows
                .iter()
                .map(|row| cell(row, at))
                .all(|value| value == MISSING || numeric(value)),
        })
        .collect();
    let rule = columns.iter().map(|column| column.width).sum::<usize>()
        + 3 * columns.len().saturating_sub(1);
    let mut out = vec![
        laid(heading, &columns, width),
        Line::from(Span::styled(
            crate::theme::glyphs::get("thin").repeat(rule.min(width)),
            crate::theme::foreground("faint").unwrap_or_default(),
        )),
    ];
    out.extend(drawn.into_iter().map(|row| laid(row, &columns, width)));
    out
}

fn cell(row: &[String], at: usize) -> &str {
    row.get(at)
        .map(String::as_str)
        .filter(|s| !s.is_empty())
        .unwrap_or(MISSING)
}

fn content(value: &str, heading: bool, width: usize) -> Line<'static> {
    if heading {
        return Line::from(Span::styled(value.to_owned(), crate::theme::ash()));
    }
    Line::from(
        crate::markdown::mentioned(value, width, &[])
            .lines
            .into_iter()
            .flat_map(|line| line.spans)
            .collect::<Vec<_>>(),
    )
}

fn laid(cells: Vec<Line<'static>>, columns: &[Column], width: usize) -> Line<'static> {
    let mut line = Line::default();
    for (at, (drawn, column)) in cells.into_iter().zip(columns).enumerate() {
        let start = line.width();
        if column.numeric {
            line.spans.push(Span::raw(
                " ".repeat(column.width.saturating_sub(drawn.width())),
            ));
        }
        line.spans.extend(drawn.spans);
        if at + 1 < columns.len() {
            line = crate::layout::grid::pad_to(line, start + column.width + 3);
        }
    }
    crate::layout::grid::fit(line, width)
}
