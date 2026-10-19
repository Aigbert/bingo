//! What an ask would do, between its title and its answers: a hunk on the
//! dock's own ground, or the whole command and what it puts at stake.
use super::spec::Body;
use crate::{
    layout::grid,
    theme,
    views::diff::{self, DiffStyle},
};
use bingo_sdk::Stakes;
use ratatui::{
    style::Style,
    text::{Line, Span},
};

/// Rows of a hunk an ask shows before it names the rest.
const DIFF_ROWS: usize = 8;

pub fn lines(body: &Body, width: usize) -> Vec<Line<'static>> {
    match body {
        Body::Diff(unified) => hunk(unified, width),
        Body::Command {
            command,
            why,
            stakes,
        } => {
            let mut out = shell(command, width);
            out.extend(why.iter().flat_map(|why| said(why, width)));
            out.extend(stakes.iter().flat_map(|stakes| staked(stakes, width)));
            out
        }
        Body::Text(text) => said(text, width),
    }
}

/// The well a diff is drawn in, without its own blank edges and on the
/// dock's ground, so only the changed rows carry a tint.
fn hunk(unified: &str, width: usize) -> Vec<Line<'static>> {
    let rows = diff::lines(unified, DiffStyle::Well);
    let more = rows.len().saturating_sub(DIFF_ROWS);
    let framed = crate::well::frame(rows.into_iter().take(DIFF_ROWS).collect(), width);
    let inner = framed.len().saturating_sub(1);
    let mut out: Vec<_> = framed.into_iter().take(inner).skip(1).map(on_you).collect();
    if more > 0 {
        out.push(at(
            grid::VX + 2,
            vec![Span::styled(
                format!("{} {more} more lines", theme::glyphs::get("ell")),
                theme::dim(),
            )],
        ));
    }
    out
}

fn on_you(mut line: Line<'static>) -> Line<'static> {
    let well = theme::ground("well").and_then(|style| style.bg);
    let you = theme::ground("you").and_then(|style| style.bg);
    for span in &mut line.spans {
        if span.style.bg == well {
            span.style.bg = you;
        }
    }
    line
}

/// The command as it will run, wrapped and never cut.
fn shell(command: &str, width: usize) -> Vec<Line<'static>> {
    let line = Line::from(vec![
        Span::styled("$ ", theme::dim()),
        Span::styled(command.to_owned(), ink("code")),
    ]);
    indented(&line, width)
}

fn said(text: &str, width: usize) -> Vec<Line<'static>> {
    text.lines()
        .flat_map(|row| {
            indented(
                &Line::from(Span::styled(row.to_owned(), ink("muted"))),
                width,
            )
        })
        .collect()
}

/// What running the command would lose, one item a row under its line.
fn staked(stakes: &Stakes, width: usize) -> Vec<Line<'static>> {
    let room = width.saturating_sub(grid::TX + grid::RM + 4);
    let mut out = vec![at(
        grid::TX + 2,
        vec![Span::styled(stakes.line.clone(), ink("bad"))],
    )];
    out.extend(stakes.items.iter().map(|item| {
        let split = item.char_indices().nth(7).map_or(item.len(), |(at, _)| at);
        let (head, rest) = item.split_at(split);
        let line = Line::from(vec![
            Span::styled(head.to_owned(), ink("muted")),
            Span::styled(rest.to_owned(), theme::dim()),
        ]);
        at(grid::TX + 4, grid::fit(line, room).spans)
    }));
    out
}

fn indented(line: &Line<'static>, width: usize) -> Vec<Line<'static>> {
    let room = width.saturating_sub(grid::TX + grid::RM + 2);
    crate::wrap::wrap(line, room)
        .into_iter()
        .map(|row| at(grid::TX + 2, row.spans))
        .collect()
}

fn at(column: usize, spans: Vec<Span<'static>>) -> Line<'static> {
    let mut line = Line::from(" ".repeat(column));
    line.spans.extend(spans);
    line
}

fn ink(name: &str) -> Style {
    theme::foreground(name).unwrap_or_default()
}
