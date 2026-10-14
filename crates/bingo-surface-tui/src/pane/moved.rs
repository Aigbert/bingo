//! The person's preferences live only in this surface process.
use super::{seat, wanted};
use crate::ui::Ui;
use bingo_sdk::{SessionState, slot::Place};

pub fn label(place: Place) -> &'static str {
    match place {
        Place::Above => "above ▾",
        Place::Side => "side ▾",
        Place::Overlay => "overlay ▾",
        Place::Rect(_) => "rect ▾",
    }
}
pub fn cycle(ui: &mut Ui, state: &SessionState, key: &str) {
    let wanted = wanted(ui, state);
    let Some(pane) = wanted.iter().find(|pane| pane.key == key) else {
        return;
    };
    let current = ui
        .shown
        .seating
        .panes
        .iter()
        .find(|pane| pane.key == key)
        .map(|pane| pane.place);
    let choices = [Place::Above, Place::Side, Place::Overlay]
        .into_iter()
        .chain(
            pane.place
                .iter()
                .copied()
                .filter(|place| matches!(place, Place::Rect(_))),
        )
        .collect::<Vec<_>>();
    let start = current
        .and_then(|place| choices.iter().position(|one| *one == place))
        .unwrap_or(choices.len() - 1);
    for step in 1..=choices.len() {
        let choice = choices[(start + step) % choices.len()];
        let mut moved = ui.placed.panes.moved.clone();
        moved.insert(key.into(), choice);
        let seating = seat::seat(ui.shown.seating.area, &wanted, &moved);
        if seating
            .panes
            .iter()
            .any(|pane| pane.key == key && pane.place == choice)
        {
            ui.placed.panes.moved.insert(key.into(), choice);
            return;
        }
    }
}
