//! The Welcome site shares the frame's transcript projection.
use crate::{layout::Layout, transcript::Rows};
use ratatui::{layout::Rect, widgets::Paragraph};
pub(super) fn render(rows: &Rows<'_>, frame: &mut Layout, area: Rect) {
    if !rows.state.transcript.items().is_empty() {
        return;
    }
    if let Some(key) = crate::component::welcome::request(rows) {
        frame.pulls.push(key);
    }
    let lines = frame.collect(crate::component::welcome::draw(rows));
    frame.render_widget(Paragraph::new(lines), area);
}
