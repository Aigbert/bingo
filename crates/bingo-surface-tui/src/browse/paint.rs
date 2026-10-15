use super::*;
use crate::fold::{Fold, Folds};
pub fn folds(ui: &Ui, tree: &Tree) -> Folds {
    let mut folds = ui.folds.clone();
    if active(ui)
        && let Some(props) = current(ui)
        && let Some(hit) = props.hit.as_ref()
    {
        folds.insert(anchor(tree.viewed(), &hit.item), Fold::Open);
    }
    folds
}
fn anchor(state: &SessionState, id: &ItemId) -> ItemId {
    crate::thoughts::run_of(state, id)
        .last()
        .map_or_else(|| id.clone(), |item| item.id.clone())
}
pub fn top(ui: &Ui, tree: &Tree, frame: &crate::layout::Layout, rows: usize) -> Option<usize> {
    if !active(ui) {
        return None;
    }
    let props = current(ui)?;
    let item = props.hit.as_ref().map(|m| m.item.clone()).or_else(|| {
        let turn = props
            .turns
            .iter()
            .find(|t| Some(&t.id) == props.selected.as_ref())?;
        tree.viewed()
            .transcript
            .items()
            .get(turn.first)
            .map(|i| i.id.clone())
    })?;
    let item = anchor(tree.viewed(), &item);
    let (start, end) = frame.blocks.span(&item)?;
    let offset = matching_line(&props, tree.viewed(), frame, start, end);
    Some((start + offset).min(frame.height.saturating_sub(rows)))
}
fn matching_line(
    props: &BrowseProps,
    state: &SessionState,
    frame: &crate::layout::Layout,
    start: usize,
    end: usize,
) -> usize {
    if props.query.is_empty() {
        return 0;
    }
    let ordinal = props
        .hit
        .as_ref()
        .and_then(|hit| {
            state.transcript.get(&hit.item).map(|item| {
                transcript::text(item)
                    .lines()
                    .take(hit.line)
                    .filter(|line| line.to_lowercase().contains(&props.query.to_lowercase()))
                    .count()
            })
        })
        .unwrap_or(0);
    let query = props.query.to_lowercase();
    frame
        .blocks
        .window(start, end - start)
        .iter()
        .enumerate()
        .filter(|(_, line)| line.to_string().to_lowercase().contains(&query))
        .nth(ordinal)
        .map_or(0, |(at, _)| at)
}
pub fn mark(
    ui: &Ui,
    tree: &Tree,
    frame: &crate::layout::Layout,
    lines: &mut [ratatui::text::Line<'static>],
) {
    if !active(ui) {
        return;
    }
    let Some(props) = current(ui) else {
        return;
    };
    for (offset, line) in lines.iter_mut().enumerate() {
        let item = frame
            .blocks
            .at(frame.top + offset)
            .and_then(|id| tree.viewed().transcript.get(&id));
        if item.is_some_and(|i| i.turn == props.selected)
            && let Some(span) = line.spans.first_mut()
        {
            span.style = span.style.patch(crate::theme::bold());
        }
        if !props.query.is_empty()
            && line
                .to_string()
                .to_lowercase()
                .contains(&props.query.to_lowercase())
        {
            line.style = line.style.patch(crate::theme::bold());
        }
    }
}
