//! Styled children share their parent's style unless a prop overrides it.
use crate::theme;
use bingo_sdk::{
    View,
    view::{TextChild, TextStyle},
};
use ratatui::{
    style::Style,
    text::{Line, Span},
};

pub fn lines(text: &str) -> Vec<Line<'static>> {
    styled(
        &[TextChild::String(text.into())],
        &TextStyle::default(),
        usize::MAX,
    )
}
pub fn styled(children: &[TextChild], props: &TextStyle, width: usize) -> Vec<Line<'static>> {
    let mut out = vec![Line::default()];
    append(children, style(theme::text(), props), &mut out);
    if out.len() > 1
        && out
            .last()
            .is_some_and(|line| line.spans.iter().all(|span| span.content.is_empty()))
    {
        out.pop();
    }
    if props.wrap.as_deref() == Some("wrap") && width > 0 {
        crate::wrap::wrap_all(&out, width)
    } else {
        out
    }
}
fn append(children: &[TextChild], inherited: Style, out: &mut Vec<Line<'static>>) {
    for child in children {
        match child {
            TextChild::String(text) => {
                for (index, part) in text.split('\n').enumerate() {
                    if index > 0 {
                        out.push(Line::default());
                    }
                    if let Some(line) = out.last_mut() {
                        line.spans.push(Span::styled(part.to_string(), inherited));
                    }
                }
            }
            TextChild::Node(node) => match node.as_ref() {
                View::Text {
                    children,
                    style: props,
                } => append(children, style(inherited, props), out),
                other => append(&[TextChild::String(other.fold())], inherited, out),
            },
        }
    }
}
pub fn color(style: Style, color: Option<&str>) -> Style {
    if theme::current().colors == theme::Colors::Plain {
        return style;
    }
    if color == Some("glow") {
        return style.patch(theme::glow());
    }
    if color == Some("ash") {
        return style.patch(theme::ash());
    }
    if color == Some("attention") {
        return style.patch(super::badge::style(bingo_sdk::Tone::Attention));
    }
    color
        .and_then(|name| name.parse().ok())
        .map_or(style, |color| style.fg(color))
}
fn style(mut style: Style, props: &TextStyle) -> Style {
    style = color(style, props.color.as_deref());
    for (setting, token) in [
        (props.dim_color, theme::dim()),
        (props.bold, theme::bold()),
        (props.italic, theme::italic()),
        (props.underline, Style::new().underlined()),
        (props.strikethrough, theme::struck()),
        (props.inverse, Style::new().reversed()),
    ] {
        style = match setting {
            Some(true) => style.patch(token),
            Some(false) => style.remove_modifier(token.add_modifier),
            None => style,
        };
    }
    style
}
