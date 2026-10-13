use crate::theme;
use bingo_sdk::view::InputProps;
use ratatui::text::{Line, Span};
pub fn lines(props: &InputProps, width: usize) -> Vec<Line<'static>> {
    let mut spans = Vec::new();
    if let Some(label) = &props.label {
        spans.push(Span::styled(format!("{label}: "), theme::text()));
    }
    let (value, style) = match props.value.as_deref().filter(|text| !text.is_empty()) {
        Some(value) => (value, theme::text()),
        None => (
            props.placeholder.as_deref().unwrap_or_default(),
            theme::dim(),
        ),
    };
    spans.push(Span::styled(value.to_string(), style));
    vec![super::fit(Line::from(spans), width)]
}
