//! A focused pane must have a seat and a readable view before input can leave its queue.
use super::*;

pub(crate) fn waiting(ui: &Ui, state: &SessionState) -> bool {
    let Focus::Pane(focused) = &ui.focus else {
        return false;
    };
    let Some(pane) = placed(state)
        .into_iter()
        .find(|pane| &pane.key == focused && pane.asked)
    else {
        return false;
    };
    let Some(seated) = ui
        .shown
        .seating
        .panes
        .iter()
        .find(|seat| &seat.key == focused)
    else {
        return true;
    };
    crate::component::pane::matched(state, &shown(ui, &pane, seated))
        && view(ui, state, &pane, seated).is_none()
}
