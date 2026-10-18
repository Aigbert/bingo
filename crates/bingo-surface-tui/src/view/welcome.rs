//! The Welcome site remains available to its owner; M199 draws its wordmark.
use crate::{clock::Now, layout::Layout, ui::Ui};
use ratatui::{layout::Rect, widgets::Paragraph};
pub(super) fn render(
    state: &bingo_sdk::SessionState,
    ui: &Ui,
    frame: &mut Layout,
    area: Rect,
    now: Now,
) {
    if !state.transcript.items().is_empty() {
        return;
    }
    let commands = ui.catalogs.specs();
    let rows = crate::transcript::Rows::of(
        state,
        area.width as usize,
        &ui.folds,
        &commands,
        &frame.measurements,
        &ui.linked,
        now,
    )
    .components(&ui.placed.pulled)
    .saying(ui.welcome.update.as_deref())
    .entrance(ui.welcome.resume.as_ref());
    if let Some(key) = crate::component::welcome::request(&rows) {
        frame.pulls.push(key);
    }
    let drawing = crate::component::welcome::draw(&rows);
    let lines = frame.collect(drawing);
    frame.render_widget(Paragraph::new(lines), area);
}
