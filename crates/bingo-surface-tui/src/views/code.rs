//! `View::Code`: syntax-highlighted on a raised band of its own width.
//! The language shares the first row's right edge. Code never wraps; past
//! eight lines the indent becomes a line-number gutter.
//!
//! A block whose language is a diff is a diff: it goes to the one unified-diff
//! renderer and wears its tints, so a patch reads the same whether a plugin
//! published it, a card previews it or an answer fenced it.

use ratatui::text::{Line, Span};
use unicode_width::UnicodeWidthStr;

use crate::theme;
use crate::views::clip;
use crate::{highlight, preview};

/// Rows from which a block is worth numbering.
const NUMBERED: usize = 8;
/// The indent of an unnumbered block, and the narrowest number gutter.
const GUTTER: usize = 4;

pub fn lines(lang: Option<&str>, text: &str, width: usize) -> Vec<Line<'static>> {
    let lang = lang.unwrap_or_default();
    if is_diff(lang) {
        return preview::diff(text);
    }
    let rows = highlight::lines(lang, text);
    let numbered = gutter(rows.len());
    let lead = if numbered { GUTTER + 2 } else { GUTTER };
    let longest = rows.iter().map(Line::width).max().unwrap_or(0);
    let band = (lead + longest + if lang.is_empty() { 0 } else { lang.width() + 2 }).min(width);
    rows.into_iter()
        .enumerate()
        .map(|(n, row)| {
            let label = if n == 0 { lang } else { "" };
            band_line(numbered.then_some(n + 1), row, band, label)
        })
        .collect()
}

pub fn numbered(
    lang: Option<&str>,
    text: &str,
    start: Option<u32>,
    width: usize,
) -> Vec<Line<'static>> {
    let Some(start) = start else {
        return lines(lang, text, width);
    };
    text.lines()
        .enumerate()
        .map(|(i, text)| {
            let number = start.saturating_add(u32::try_from(i).unwrap_or(u32::MAX));
            crate::layout::grid::fit(
                Line::from(vec![
                    Span::styled(format!("{number:>4}   "), theme::dim()),
                    Span::styled(text.to_owned(), theme::ash()),
                ]),
                width,
            )
        })
        .collect()
}

fn band_line(
    number: Option<usize>,
    row: Line<'static>,
    width: usize,
    label: &str,
) -> Line<'static> {
    let label = clip(label, width.saturating_sub(GUTTER + 2));
    let room = width.saturating_sub(if label.is_empty() {
        0
    } else {
        label.width() + 2
    });
    let mut row = line(number, row, room);
    let padding = width.saturating_sub(row.width() + label.width());
    row.spans.push(Span::raw(" ".repeat(padding)));
    if !label.is_empty() {
        row.spans.push(Span::styled(label, theme::dim()));
    }
    for span in &mut row.spans {
        span.style = theme::raised().patch(span.style);
    }
    row.style = theme::raised();
    row
}

/// The two words that mean a block is a patch rather than a program.
fn is_diff(lang: &str) -> bool {
    matches!(lang, "diff" | "patch")
}

/// Whether the block is long enough to number.
fn gutter(rows: usize) -> bool {
    rows > NUMBERED
}

fn line(number: Option<usize>, row: Line<'static>, width: usize) -> Line<'static> {
    let lead = match number {
        Some(n) => format!("{n:>GUTTER$}  "),
        None => " ".repeat(GUTTER),
    };
    let lead = clip(&lead, width);
    let room = width.saturating_sub(lead.width());
    let mut spans = vec![Span::styled(lead, theme::dim())];
    spans.extend(cut(row, room));
    Line::from(spans)
}

/// A highlighted row, cut to the cells it has. Code never wraps (design §7):
/// what does not fit is elided, and the whole of it opens in the pager.
fn cut(row: Line<'static>, width: usize) -> Vec<Span<'static>> {
    if row
        .spans
        .iter()
        .map(|span| span.content.width())
        .sum::<usize>()
        <= width
    {
        return row.spans;
    }
    let mut out: Vec<Span<'static>> = Vec::new();
    let mut used = 0usize;
    for span in row.spans {
        if used >= width {
            break;
        }
        let text = clip(&span.content, width - used);
        used += text.width();
        out.push(Span::styled(text, span.style));
    }
    out
}

/// Answer code fills the measure with a separate language row and no gutter.
pub fn answer_fence(lang: Option<&str>, text: &str, width: usize) -> Vec<Line<'static>> {
    if is_diff(lang.unwrap_or_default()) {
        return preview::diff(text);
    }
    use crate::layout::grid;
    let ground = theme::ground("well").unwrap_or_default();
    let language = lang.unwrap_or_default();
    let mut head = Line::from(Span::raw(
        " ".repeat(width.saturating_sub(language.width() + 2)),
    ));
    head.spans
        .push(Span::styled(language.to_string(), theme::dim()));
    let mut rows = vec![grid::pad_to(head, width)];
    for line in highlight::lines(language, text) {
        let code = grid::fit(line, width.saturating_sub(4));
        let mut row = Line::from("  ");
        row.spans.extend(code.spans);
        rows.push(grid::pad_to(row, width));
    }
    rows.push(Line::from(" ".repeat(width)));
    for row in &mut rows {
        for span in &mut row.spans {
            span.style = ground.patch(span.style);
        }
    }
    rows
}
