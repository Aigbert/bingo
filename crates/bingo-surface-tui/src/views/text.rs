//! Styled children share their parent's style unless a prop overrides it.
use crate::theme;
use bingo_sdk::{
    View,
    view::{TextChild, TextMotion, TextStyle},
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
    animated(children, props, width, None)
}

pub fn animated(
    children: &[TextChild],
    props: &TextStyle,
    width: usize,
    now: Option<crate::clock::Now>,
) -> Vec<Line<'static>> {
    let mut out = vec![Line::default()];
    append_node(children, props, theme::text(), now, &mut out);
    if out.len() > 1
        && out
            .last()
            .is_some_and(|line| line.spans.iter().all(|span| span.content.is_empty()))
    {
        out.pop();
    }
    let out = if props.wrap.as_deref() == Some("wrap") && width > 0 {
        crate::wrap::wrap_all(&out, width)
    } else {
        out
    };
    out.into_iter()
        .map(|line| fill(line, props.ground.as_deref(), width))
        .collect()
}
fn append(
    children: &[TextChild],
    inherited: Style,
    now: Option<crate::clock::Now>,
    out: &mut Vec<Line<'static>>,
) {
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
                } => append_node(children, props, inherited, now, out),
                other => append(&[TextChild::String(other.fold())], inherited, now, out),
            },
        }
    }
}
pub fn color(style: Style, color: Option<&str>) -> Style {
    if color == Some("attention") {
        return style.patch(super::badge::style(bingo_sdk::Tone::Attention));
    }
    color
        .and_then(theme::foreground)
        .map_or(style, |token| style.patch(token))
}

fn style(mut style: Style, props: &TextStyle) -> Style {
    style = color(style, props.color.as_deref());
    if let Some(ground) = props.ground.as_deref().and_then(theme::ground) {
        style = style.patch(ground);
    }
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

fn append_node(
    children: &[TextChild],
    props: &TextStyle,
    inherited: Style,
    now: Option<crate::clock::Now>,
    out: &mut Vec<Line<'static>>,
) {
    let style = super::motion::style(style(inherited, props), props.motion.as_deref(), now);
    if props.motion.as_deref() == Some(&TextMotion::Spin) {
        let glyph = now
            .map(crate::spinner::glyph)
            .unwrap_or_else(|| theme::glyphs::get("still"));
        append(&[TextChild::String(glyph.to_string())], style, now, out);
    } else {
        append(children, style, now, out);
    }
}

pub fn fill(line: Line<'static>, ground: Option<&str>, width: usize) -> Line<'static> {
    let Some(ground) = ground.and_then(theme::ground) else {
        return line;
    };
    let mut line = if width < usize::MAX {
        super::fit(line, width)
    } else {
        line
    };
    for span in &mut line.spans {
        span.style = ground.patch(span.style);
    }
    line
}

pub fn next(
    children: &[TextChild],
    props: &TextStyle,
    now: crate::clock::Now,
) -> Option<std::time::Instant> {
    props
        .motion
        .as_deref()
        .and_then(|motion| super::motion::next(motion, now))
        .into_iter()
        .chain(children.iter().filter_map(|child| match child {
            TextChild::Node(node) => super::next_change(node, now),
            _ => None,
        }))
        .min()
}
