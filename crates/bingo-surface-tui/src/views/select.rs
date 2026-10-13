use crate::theme;
use bingo_sdk::view::SelectProps;
use ratatui::text::{Line, Span};
pub fn lines(props: &SelectProps, width: usize) -> Vec<Line<'static>> {
    let mut lines = Vec::new();
    if let Some(label) = &props.label {
        lines.push(Line::from(Span::styled(label.clone(), theme::text())));
    }
    for option in props.options.as_deref().unwrap_or_default() {
        let selected = props.value.as_deref() == Some(&option.value);
        lines.push(super::fit(
            Line::from(vec![
                Span::styled(
                    if selected {
                        format!("{} ", theme::point())
                    } else {
                        "  ".into()
                    },
                    theme::presence(),
                ),
                Span::styled(option.label.clone(), theme::text()),
            ]),
            width,
        ));
    }
    lines
}
