//! Focus eligibility comes from the same drawing that receives keys.
use super::{Pane, controls, seat::Seated, view};
use crate::{focus::Focus, ui::Ui};
use bingo_sdk::{SessionState, View};

pub fn focusable(ui: &Ui, state: &SessionState, pane: &Pane, seat: &Seated) -> bool {
    pane.asked && view(ui, state, pane, seat).is_some_and(takes_keys)
}

fn takes_keys(view: &View) -> bool {
    if matches!(view, View::Box { layout, .. } if layout.on_key.is_some()) {
        return true;
    }
    let mut items = Vec::new();
    controls(view, &mut items);
    !items.is_empty()
}

pub fn keys(ui: &Ui, state: &SessionState) -> Vec<String> {
    let panes = super::wanted(ui, state);
    ui.shown
        .seating
        .panes
        .iter()
        .filter_map(|seat| {
            panes
                .iter()
                .find(|pane| pane.key == seat.key)
                .filter(|pane| focusable(ui, state, pane, seat))
                .map(|pane| pane.key.clone())
        })
        .collect()
}

/// Controls keep their local order; at either edge Tab leaves the pane.
pub fn tab_control(ui: &mut Ui, state: &SessionState, backwards: bool) -> bool {
    let Focus::Pane(key) = &ui.focus else {
        return false;
    };
    let Some(pane) = super::placed(state)
        .into_iter()
        .find(|pane| &pane.key == key)
    else {
        return false;
    };
    let Some(seat) = ui.shown.seating.panes.iter().find(|seat| &seat.key == key) else {
        return false;
    };
    let Some(view) = view(ui, state, &pane, seat) else {
        return false;
    };
    let mut items = Vec::new();
    controls(view, &mut items);
    let selected = ui.placed.panes.selected.entry(key.clone()).or_default();
    let next = if backwards {
        selected.checked_sub(1)
    } else {
        selected.checked_add(1)
    };
    if let Some(next) = next.filter(|next| *next < items.len()) {
        *selected = next;
        true
    } else {
        false
    }
}
