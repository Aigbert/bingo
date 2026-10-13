use super::Marks;
use crate::theme;
use bingo_sdk::{
    ActionItem,
    view::{ButtonProps, TextChild},
};
use ratatui::text::Line;
pub fn lines(
    children: &[TextChild],
    props: &ButtonProps,
    width: usize,
    marks: &Marks,
) -> Vec<Line<'static>> {
    let label = props
        .label
        .clone()
        .unwrap_or_else(|| bingo_sdk::view::text_children(children));
    let mut lines = if let Some(action) = &props.on_press {
        super::actions::lines(
            &[ActionItem {
                label,
                action: action.clone(),
                key: props.hotkey,
            }],
            width,
            marks,
        )
    } else {
        super::text::lines(&format!("[ {label} ]"))
    };
    let style = match props.variant.as_deref() {
        Some("danger") => theme::bad(),
        Some("primary") => theme::presence(),
        _ => Default::default(),
    };
    for line in &mut lines {
        for span in &mut line.spans {
            span.style = span.style.patch(style);
            if props.dim_color == Some(true) {
                span.style = span.style.patch(theme::dim());
            }
        }
    }
    lines
}
