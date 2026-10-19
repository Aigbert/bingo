//! Selection belongs to the UI; its turn and rows belong to the transcript.
use crate::{clock::Now, focus::Focus, keys, ui::Ui};
use bingo_sdk::{Item, ItemBody, ItemId, Level, SessionState, TurnId};
use crossterm::event::KeyEvent;

pub fn row(item: &Item) -> bool {
    match &item.body {
        ItemBody::Reasoning { text, .. } => !text.trim().is_empty(),
        ItemBody::ToolCall {
            asked_at,
            began_at,
            quiet,
            output,
            ..
        } => {
            (!*quiet || item.status.failed(output.as_deref()))
                && (asked_at.is_some()
                    || began_at.is_some()
                    || item.status.is_terminal()
                        && item.status != bingo_sdk::ItemStatus::Interrupted)
        }
        _ => false,
    }
}

fn rows<'a>(state: &'a SessionState, turn: &TurnId) -> Vec<&'a Item> {
    state
        .transcript
        .items()
        .iter()
        .map(AsRef::as_ref)
        .filter(|item| item.turn.as_ref() == Some(turn) && row(item))
        .collect()
}

pub fn open(ui: &mut Ui, state: &SessionState, frame: &crate::layout::Layout, now: Now) {
    let candidate = state.transcript.items().iter().rev().find(|item| {
        row(item)
            && item.turn.is_some()
            && state
                .turn
                .as_ref()
                .is_none_or(|turn| Some(&turn.id) != item.turn.as_ref())
    });
    let Some(turn) = candidate.and_then(|item| item.turn.as_ref()) else {
        ui.notify(Level::Info, "no steps to show yet", now.instant);
        return;
    };
    ui.scroll = crate::scroll::Scroll::Offset(frame.below());
    ui.set_well(turn.clone(), None);
    if let Some(first) = rows(state, turn).first() {
        ui.focus = Focus::Detail(first.id.clone());
    }
}

pub fn close(ui: &mut Ui, tree: &crate::tree::Tree, now: Now) {
    let state = tree.viewed();
    if let Some(turn) = crate::turns::detail(state, ui.focus.detail()).cloned() {
        ui.set_well(turn, None);
    }
    let below = ui.shown.below();
    ui.focus = Focus::Composer;
    ui.scroll = crate::scroll::Scroll::Offset(below);
    let closed = crate::layout::layout(
        Default::default(),
        tree,
        ui,
        ui.shown.canvas.area(),
        now,
        &crate::render::current(),
    );
    // Folding can bring the foot into view: a closed turn that reaches it follows.
    ui.scroll = if closed.below() == 0 {
        crate::scroll::Scroll::Tail(None)
    } else {
        crate::scroll::Scroll::park(closed.top, now.instant)
    };
}

pub fn toggle(ui: &mut Ui, state: &SessionState, id: &ItemId) {
    let Some(item) = state
        .transcript
        .get(id)
        .filter(|item| item.status.is_terminal() && row(item))
    else {
        return;
    };
    let Some(turn) = item.turn.as_ref() else {
        return;
    };
    let next = (ui.wells.get(turn) != Some(id)).then(|| id.clone());
    ui.set_well(turn.clone(), next);
}

pub fn key(ui: &mut Ui, tree: &crate::tree::Tree, key: KeyEvent, now: Now) -> bool {
    let state = tree.viewed();
    match keys::detail(key) {
        Some(keys::Detail::Close) => close(ui, tree, now),
        Some(keys::Detail::Find) => crate::search::open(ui, now),
        Some(keys::Detail::Open) => {
            if let Some(id) = ui.focus.detail().cloned() {
                toggle(ui, state, &id);
            }
        }
        Some(keys::Detail::Move(by)) => move_by(ui, state, by),
        None => return false,
    }
    true
}

fn move_by(ui: &mut Ui, state: &SessionState, by: isize) {
    let Some(turn) = crate::turns::detail(state, ui.focus.detail()) else {
        return;
    };
    let rows = rows(state, turn);
    let at = rows
        .iter()
        .position(|item| Some(&item.id) == ui.focus.detail())
        .unwrap_or(0);
    if let Some(item) = rows.get(
        at.saturating_add_signed(by)
            .min(rows.len().saturating_sub(1)),
    ) {
        ui.focus = Focus::Detail(item.id.clone());
    }
}

pub fn top(ui: &Ui, frame: &crate::layout::Layout, height: usize) -> usize {
    let Some((line, _)) = ui.focus.detail().and_then(|id| frame.blocks.span(id)) else {
        return frame.top;
    };
    let top = frame.top;
    let end = top + height;
    if line < top + 1 {
        line.saturating_sub(1)
    } else if line >= end.saturating_sub(2) {
        line.saturating_add(3).saturating_sub(height)
    } else {
        top
    }
    .min(frame.height.saturating_sub(height))
}

#[cfg(test)]
mod tests;
