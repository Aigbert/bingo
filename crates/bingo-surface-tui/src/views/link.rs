use crate::theme;
use ratatui::text::{Line, Span};
pub fn lines(href: &str, label: Option<&str>, width: usize) -> Vec<Line<'static>> {
    let mut spans = vec![Span::styled(
        label.unwrap_or(href).to_string(),
        theme::link(),
    )];
    if label.is_some_and(|label| label != href) {
        spans.push(Span::styled(format!(" ({href})"), theme::dim()));
    }
    crate::wrap::wrap(&Line::from(spans), width)
}
