//! A turn's parts, every one at column 3 (ADR-0095 §6–§8): a step's row, the
//! line a finished turn folds to, a thought of an opened turn, and the line
//! where the person stopped it. What hangs under a step starts at column 5,
//! under its words.

use bingo_sdk::component::ledger::{Marks, STEP_INDENT, plural};
use bingo_sdk::component::row::{Ink, Piece};
use bingo_sdk::component::{Props, Step, StepState, TurnFoldProps};

use super::*;
use crate::turns::Role;

/// Where what hangs under a step starts: under the step's own words.
const HANG: usize = 5;

/// The lines an item draws for the part it plays in its turn, or nothing at
/// all when the part is its own drawing.
pub(super) fn part(item: &Item, role: &Role, rows: &Rows<'_>) -> Option<Block> {
    match role {
        Role::Own => None,
        Role::Hidden => Some(Block::default()),
        Role::Step { step, opened } => Some(step_block(item, step, *opened, rows)),
        Role::Fold { fold, then } => Some(fold_block(item, fold, then.as_deref(), rows)),
        Role::Thought => Some(thought_block(item, rows).into()),
        Role::Stop { ran, total } => Some(stop_block(item, *ran, *total, rows).into()),
    }
}

/// A step's row, through the site that owns it, and, under an opened turn,
/// what the step brought back: its words, then its pictures.
fn step_block(item: &Item, step: &Step, opened: bool, rows: &Rows<'_>) -> Block {
    let (id, props) = crate::turns::ledger(item, step);
    let mut out = crate::component::resolve(
        rows.state,
        rows.pulled,
        &id,
        &props,
        rows.width,
        &crate::views::Marks::at(rows.now),
        |changed, width| ledger(changed, width, rows.now),
    );
    // An action is on what the step is doing; a step that has ended is doing
    // nothing, whatever a producer last said.
    if matches!(step.state, StepState::Running | StepState::Asked) {
        out.lines
            .extend(super::slots::actions_under(item, HANG, rows));
    }
    if !opened {
        return out.into();
    }
    out.append(result(item, rows));
    let mut block = Block::from(out);
    pictured::under_the_step(&mut block, item, HANG, rows);
    block
}

/// The row of a step that is asking, standing on its own: the head of the
/// card it opens where its row was, in the row's own columns.
pub(crate) fn asked(step: &Step, width: usize) -> Line<'static> {
    let glyph = theme::glyphs().chrome.steps[1];
    line(step.row(glyph, width, UnicodeWidthStr::width))
}

/// The surface's own drawing of the ledger: one row a step.
fn ledger(props: &Props, width: usize, now: Now) -> Vec<Line<'static>> {
    let Props::Ledger(ledger) = props else {
        return Vec::new();
    };
    ledger
        .steps
        .iter()
        .map(|step| {
            let glyph = glyph(step, &ledger.marks, now);
            line(step.row(glyph, width, UnicodeWidthStr::width))
        })
        .collect()
}

/// What a step's row starts with: what state it is in, a running one turning.
fn glyph<'a>(step: &Step, marks: &'a Marks, now: Now) -> &'a str {
    match step.state {
        StepState::Running => crate::spinner::glyph(now),
        state => marks.step(state),
    }
}

/// The line a finished turn folds to, through the site that owns it, and
/// under it, when the turn is opened, the part the holder plays there.
fn fold_block(item: &Item, fold: &TurnFoldProps, then: Option<&Role>, rows: &Rows<'_>) -> Block {
    let (id, props) = crate::turns::folded(item, fold);
    let out = crate::component::resolve(
        rows.state,
        rows.pulled,
        &id,
        &props,
        rows.width,
        &crate::views::Marks::at(rows.now),
        folded_line,
    );
    let mut block = Block::from(out);
    if let Some(under) = then.and_then(|then| part(item, then, rows)) {
        block.next = block.next.into_iter().chain(under.next).min();
        block.lines.extend(under.lines);
        block.pictures.extend(under.pictures);
    }
    block
}

/// The surface's own drawing of the folded line.
fn folded_line(props: &Props, width: usize) -> Vec<Line<'static>> {
    let Props::TurnFold(fold) = props else {
        return Vec::new();
    };
    fold.row(width, UnicodeWidthStr::width)
        .map(line)
        .into_iter()
        .collect()
}

/// What came back from a step, through the site that owns it.
fn result(item: &Item, rows: &Rows<'_>) -> crate::views::Drawing {
    let Some((id, props)) = crate::turns::result(item) else {
        return crate::views::Drawing::default();
    };
    crate::component::resolve(
        rows.state,
        rows.pulled,
        &id,
        &props,
        rows.width,
        &crate::views::Marks::at(rows.now),
        |changed, width| {
            let Props::ToolResult(result) = changed else {
                return crate::views::Drawing::default();
            };
            let output = crate::component::items::output_of(&result.output);
            hung(
                &output,
                &Rows {
                    width,
                    ..rows.clone()
                },
            )
        },
    )
}

/// What came back from a step, under its words, nothing of it folded: the
/// turn was opened to be read.
fn hung(output: &ToolOutput, rows: &Rows<'_>) -> crate::views::Drawing {
    let inner = rows.width.saturating_sub(HANG).max(1);
    folded(output, Fold::Open, inner, Some(rows.now)).map_lines(|lines| {
        let body = lines
            .into_iter()
            .map(|line| shortened(line, rows))
            .collect();
        under(Span::raw(" ".repeat(HANG)), body, HANG, rows.width)
    })
}

/// A line of output with the paths in it read from this session's directory.
fn shortened(line: Line<'static>, rows: &Rows<'_>) -> Line<'static> {
    let spans = line
        .spans
        .into_iter()
        .map(|span| Span::styled(rows.shorten(&span.content), span.style))
        .collect::<Vec<_>>();
    Line::from(spans).style(line.style)
}

/// A thought of an opened turn, through the site that owns it.
fn thought_block(item: &Item, rows: &Rows<'_>) -> crate::views::Drawing {
    let Some((id, props)) = crate::turns::fact(item) else {
        return crate::views::Drawing::default();
    };
    crate::component::resolve(
        rows.state,
        rows.pulled,
        &id,
        &props,
        rows.width,
        &crate::views::Marks::at(rows.now),
        |changed, width| {
            let Props::Reasoning(reasoning) = changed else {
                return Vec::new();
            };
            thought(&reasoning.text, width)
        },
    )
}

/// A thought's words, dim, under the steps' words.
fn thought(text: &str, width: usize) -> Vec<Line<'static>> {
    under(Span::raw(" ".repeat(HANG)), plain(text), HANG, width)
}

/// Where the person stopped the turn, through the site that owns the stop.
/// The stop line counts the turn's steps, which the site's props do not
/// carry, so the surface's own drawing is that line whatever they say.
fn stop_block(item: &Item, ran: usize, total: usize, rows: &Rows<'_>) -> crate::views::Drawing {
    let Some((id, props)) = crate::turns::fact(item) else {
        return vec![stop_line(ran, total)].into();
    };
    crate::component::resolve(
        rows.state,
        rows.pulled,
        &id,
        &props,
        rows.width,
        &crate::views::Marks::at(rows.now),
        |_, _| vec![stop_line(ran, total)],
    )
}

/// The line a stopped turn ends on: `■ you stopped the turn · 2 of 3 steps
/// had run; nothing after them`, or the first four words when no step had
/// begun.
fn stop_line(ran: usize, total: usize) -> Line<'static> {
    let mut words = String::from("you stopped the turn");
    if total > 0 {
        words.push_str(&format!(
            " \u{b7} {ran} of {total} {} had run; nothing after them",
            plural(total)
        ));
    }
    let glyph = theme::glyphs().chrome.stop;
    Line::from(vec![
        Span::styled(STEP_INDENT, theme::text()),
        Span::styled(format!("{glyph} "), theme::bad()),
        Span::styled(words, theme::dim()),
    ])
}

/// A row of pieces in the theme's inks.
fn line(pieces: Vec<Piece>) -> Line<'static> {
    let spans = pieces
        .into_iter()
        .map(|piece| Span::styled(piece.text, ink(piece.ink)))
        .collect::<Vec<_>>();
    Line::from(spans)
}

fn ink(ink: Ink) -> Style {
    match ink {
        Ink::Plain => theme::text(),
        Ink::Dim => theme::dim(),
        Ink::Ash => theme::ash(),
        Ink::Good => theme::good(),
        Ink::Bad => theme::bad(),
        Ink::Glow => theme::glow(),
        Ink::Presence => theme::presence(),
    }
}

#[cfg(test)]
mod tests {
    use super::stop_line;

    #[test]
    fn the_stop_line_counts_what_ran_and_says_nothing_of_steps_never_begun() {
        let stop = crate::theme::glyphs().chrome.stop;
        assert_eq!(
            stop_line(2, 3).to_string(),
            format!(
                "   {stop} you stopped the turn \u{b7} 2 of 3 steps had run; nothing after them"
            )
        );
        assert_eq!(
            stop_line(0, 1).to_string(),
            format!(
                "   {stop} you stopped the turn \u{b7} 0 of 1 step had run; nothing after them"
            )
        );
        assert_eq!(
            stop_line(0, 0).to_string(),
            format!("   {stop} you stopped the turn")
        );
    }
}
