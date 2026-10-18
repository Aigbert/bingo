//! `View::Panel`: a bold title and its child indented under it — the same
//! shape a block of the transcript has, so a panel reads as one thing.

use bingo_sdk::View;
use ratatui::text::{Line, Span};

use crate::theme;
use crate::views::{Drawing, Engine, Marks, indent, marked_with};

/// How far the child hangs under the title.
const INDENT: usize = 2;

pub fn lines(
    title: &str,
    child: &View,
    width: usize,
    marks: &Marks,
    engine: &Engine<'_>,
) -> Drawing {
    let mut out = vec![Line::from(Span::styled(
        title.to_string(),
        theme::text().patch(theme::bold()),
    ))];
    marked_with(child, width.saturating_sub(INDENT), marks, engine).map_lines(|lines| {
        out.extend(lines.into_iter().map(|line| indent(line, INDENT)));
        out
    })
}
