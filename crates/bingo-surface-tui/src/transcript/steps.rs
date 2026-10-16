//! A turn's parts, every one at column 3 (ADR-0095 §6–§8): a step's row, the
//! line a finished turn folds to, a thought of an opened turn, and the line
//! where the person stopped it. What hangs under a step starts at column 5,
//! under its words.

use bingo_sdk::component::ledger::{FoldGlyphs, stop_row};
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
        Role::Thought => Some(thought(item, rows).into()),
        Role::Stop { ran, total } => {
            let glyph = theme::glyphs().chrome.stop;
            Some(vec![line(stop_row(glyph, *ran, *total))].into())
        }
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
        |changed, width| ledger(changed, width, rows.now),
    );
    out.extend(super::slots::actions_under(item, HANG, rows));
    if !opened {
        return out.into();
    }
    out.extend(hung(item, rows));
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
        .map(|step| line(step.row(glyph(step, now), width, UnicodeWidthStr::width)))
        .collect()
}

/// What a step's row starts with: what state it is in, a running one turning.
fn glyph(step: &Step, now: Now) -> &'static str {
    let steps = theme::glyphs().chrome.steps;
    match step.state {
        StepState::Running => crate::spinner::glyph(now),
        StepState::Done => steps[0],
        StepState::Asked => steps[1],
        StepState::Failed | StepState::Stopped => steps[3],
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
        folded_line,
    );
    let mut block = Block::from(out);
    if let Some(under) = then.and_then(|then| part(item, then, rows)) {
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
    let chrome = theme::glyphs().chrome;
    let glyphs = FoldGlyphs {
        folded: chrome.fold[0],
        open: chrome.fold[1],
    };
    fold.row(glyphs, width, UnicodeWidthStr::width)
        .map(line)
        .into_iter()
        .collect()
}

/// What came back from a step, under its words, nothing of it folded: the
/// turn was opened to be read.
fn hung(item: &Item, rows: &Rows<'_>) -> Vec<Line<'static>> {
    let ItemBody::ToolCall {
        output: Some(output),
        ..
    } = &item.body
    else {
        return Vec::new();
    };
    let inner = rows.width.saturating_sub(HANG).max(1);
    let body = folded(output, Fold::Open, inner)
        .into_iter()
        .map(|line| shortened(line, rows))
        .collect();
    under(Span::raw(" ".repeat(HANG)), body, HANG, rows.width)
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

/// A thought of an opened turn: its words, dim, under the steps' words.
fn thought(item: &Item, rows: &Rows<'_>) -> Vec<Line<'static>> {
    let ItemBody::Reasoning { text, .. } = &item.body else {
        return Vec::new();
    };
    under(Span::raw(" ".repeat(HANG)), plain(text), HANG, rows.width)
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
