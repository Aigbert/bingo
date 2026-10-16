//! The comet tail (§6): the cells of a block that just arrived wear
//! `presence`'s glow and cool to `text` behind them.

use super::*;

/// How many cells of the tail are still warm.
const COMET_CELLS: usize = 8;

/// How far a block still arriving is through its tail, and nothing at all
/// once the tail has cooled or where nothing may move.
pub(super) fn arriving(status: ItemStatus, rows: &Rows<'_>, cue: Cue) -> Option<f32> {
    if status != ItemStatus::Running || !rows.now.motion {
        return None;
    }
    let age = Anim::new(cue.since, COMET).progress(rows.now.instant);
    (age < 1.0).then_some(age)
}

/// The comet tail (§6): the cells that just arrived wear `presence`'s glow and
/// cool to `text` behind them. It is a style pass over the last row — the text
/// is the reducer's, and nothing here keeps a copy of it to know what is new.
pub(super) fn comet(mut body: Vec<Line<'static>>, age: f32) -> Vec<Line<'static>> {
    let Some(last) = body.pop() else {
        return body;
    };
    body.push(tail_lit(last, age));
    body
}

fn tail_lit(line: Line<'static>, age: f32) -> Line<'static> {
    let total: usize = line.spans.iter().map(|s| s.content.chars().count()).sum();
    let from = total.saturating_sub(COMET_CELLS);
    let mut out: Vec<Span<'static>> = Vec::new();
    let mut seen = 0;
    for span in line.spans {
        let count = span.content.chars().count();
        if seen + count <= from {
            seen += count;
            out.push(span);
            continue;
        }
        let keeps = from.saturating_sub(seen);
        let head: String = span.content.chars().take(keeps).collect();
        if !head.is_empty() {
            out.push(Span::styled(head, span.style));
        }
        for (i, c) in span.content.chars().enumerate().skip(keeps) {
            out.push(Span::styled(
                c.to_string(),
                cooling(total - (seen + i) - 1, age),
            ));
        }
        seen += count;
    }
    Line::from(out)
}

/// One cell of the tail: the newest is the warmest, and each cell behind it is
/// that much further along the same ramp.
fn cooling(back: usize, age: f32) -> Style {
    let behind = back as f32 / COMET_CELLS as f32;
    theme::comet((age + behind).min(1.0))
}
