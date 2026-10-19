//! What is under a finished or running tool row: the text as it arrived,
//! folded to the rows the transcript can spare, with a line that says how
//! much was left out and what opens it (design §5).

use bingo_sdk::{ContentPart, ToolOutput};
use ratatui::text::{Line, Span};
use unicode_width::UnicodeWidthChar;

use crate::fold::Fold;
use crate::{theme, views, wrap};

/// The last `keep` rows of something still arriving: what a running tool has
/// printed so far, or what a thought has been thinking. One tail, so the two
/// move the same way (§6); how many rows each spends is the one thing they
/// differ by, and each names its own.
///
/// Rows as the transcript will draw them, which is what `width` is for. A
/// paragraph is one logical line and any number of rows, so counting lines let
/// the block change height on every delta — and the transcript is anchored at
/// its foot, so that moved everything above it (2026-09-06). Only the last
/// `keep` logical lines are wrapped: each holds at least one row, so `keep` of
/// them always hold the `keep` rows the cut needs, and the work per delta stays
/// with the tail instead of the whole text.
///
/// Blank lines are left out before that cut, because a tail is the newest of
/// what has *arrived* and a paragraph break is not something that arrived: a
/// thought of three paragraphs whose last was one short sentence held its `⎿`
/// on the break with the sentence under it, the connector pointing at an empty
/// row for the rest of the session (2026-09-06, the hands-on drive). A line
/// with text on it wraps to rows that carry that text, so the tail is `keep`
/// rows of text wherever the arriving text has `keep` non-blank lines to give,
/// and shorter only where it has fewer.
pub(super) fn tail(arriving: &str, keep: usize, width: usize) -> Vec<Line<'static>> {
    let lines: Vec<&str> = arriving
        .lines()
        .filter(|line| !line.trim().is_empty())
        .collect();
    let last = lines[lines.len().saturating_sub(keep)..].join("\n");
    let mut rows = wrap::wrap_all(&plain(&last), width);
    rows.split_off(rows.len().saturating_sub(keep))
}

/// The full owner's display for Browse's reader.
pub fn whole(output: &ToolOutput, width: usize) -> Vec<Line<'static>> {
    match &output.display {
        Some(view) => views::render(view, width),
        None => plain(&text_of(output)),
    }
}

pub(super) fn plain(text: &str) -> Vec<Line<'static>> {
    text.trim_end()
        .lines()
        .map(|line| Line::from(Span::styled(expand_tabs(line), theme::dim())))
        .collect()
}

/// A terminal cell has no tab in it: each one runs to the next stop of eight,
/// as the shell would have shown it.
pub(super) fn expand_tabs(line: &str) -> String {
    let mut out = String::with_capacity(line.len());
    let mut column = 0;
    for c in line.chars() {
        if c == '\t' {
            let stop = 8 - column % 8;
            out.extend(std::iter::repeat_n(' ', stop));
            column += stop;
        } else {
            out.push(c);
            column += UnicodeWidthChar::width(c).unwrap_or(0);
        }
    }
    out
}

pub(super) fn text_of(output: &ToolOutput) -> String {
    output
        .parts
        .iter()
        .filter_map(ContentPart::as_text)
        .collect::<Vec<_>>()
        .join("\n")
}

/// What a block shows under its row, from the one fold it is in: nothing, the
/// first `limit` rows with how many were left out, or the whole of it. One map
/// answers for every fold, so a block is open in one way only.
pub(super) fn kept(
    rows: Vec<Line<'static>>,
    fold: Fold,
    limit: usize,
    opens: Option<&str>,
) -> Vec<Line<'static>> {
    match fold {
        Fold::Shut => Vec::new(),
        Fold::Peek => cut(rows, limit, opens),
        Fold::Open => rows,
    }
}

/// The first rows, then how many were left out and what opens them. `opens` is
/// `None` for what no key reaches: `ctrl+o` reaches a result, so a block that
/// is not one says how much it kept back and promises nothing.
pub(super) fn cut(
    rows: Vec<Line<'static>>,
    limit: usize,
    opens: Option<&str>,
) -> Vec<Line<'static>> {
    let hidden = rows.len().saturating_sub(limit);
    let mut out: Vec<Line<'static>> = rows.into_iter().take(limit).collect();
    if hidden > 0 {
        let key = opens.map(|key| format!(" ({key})")).unwrap_or_default();
        out.push(Line::from(Span::styled(
            format!("{} +{hidden} lines{key}", theme::ellipsis()),
            theme::dim(),
        )));
    }
    out
}
