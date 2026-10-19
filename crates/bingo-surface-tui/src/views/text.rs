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
    animated(children, props, width, None).lines
}

#[derive(Clone, Copy)]
struct InkStyle {
    style: Style,
    ink: ratatui::style::Color,
}

pub fn animated(
    children: &[TextChild],
    props: &TextStyle,
    width: usize,
    now: Option<crate::clock::Now>,
) -> super::Drawing {
    let mut out = super::Drawing::still(vec![Line::default()]);
    let inherited = InkStyle {
        style: theme::text(),
        ink: theme::palette().fg(),
    };
    append_node(children, props, inherited, now, &mut out);
    out.map_lines(|mut lines| {
        if lines.len() > 1
            && lines
                .last()
                .is_some_and(|line| line.spans.iter().all(|span| span.content.is_empty()))
        {
            lines.pop();
        }
        let lines = if props.wrap.as_deref() == Some("wrap") && width > 0 {
            crate::wrap::wrap_all(&lines, width)
        } else {
            lines
        };
        lines
            .into_iter()
            .map(|line| fill(line, props.ground.as_deref(), width))
            .collect()
    })
}
fn append(
    children: &[TextChild],
    inherited: InkStyle,
    now: Option<crate::clock::Now>,
    out: &mut super::Drawing,
) {
    for child in children {
        match child {
            TextChild::String(text) => {
                for (index, part) in text.split('\n').enumerate() {
                    if index > 0 {
                        out.lines.push(Line::default());
                    }
                    if let Some(line) = out.lines.last_mut() {
                        line.spans
                            .push(Span::styled(part.to_string(), inherited.style));
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
    inherited: InkStyle,
    now: Option<crate::clock::Now>,
    out: &mut super::Drawing,
) {
    let ink = if props.dim_color == Some(true) {
        theme::palette().dim()
    } else {
        props
            .color
            .as_deref()
            .and_then(|name| theme::palette().named(name))
            .unwrap_or(inherited.ink)
    };
    let style = InkStyle {
        style: super::motion::style(
            style(inherited.style, props),
            ink,
            props.motion.as_deref(),
            now,
        ),
        ink,
    };
    let next = now.and_then(|now| {
        props
            .motion
            .as_deref()
            .and_then(|motion| super::motion::next(motion, now))
    });
    out.next = out.next.into_iter().chain(next).min();
    if let Some(text) = super::motion::countdown(props.motion.as_deref(), now, children) {
        append(&[TextChild::String(text)], style, now, out);
    } else if props.motion.as_deref() == Some(&TextMotion::Spin) {
        let glyph = now
            .map(crate::spinner::glyph)
            .unwrap_or_else(|| theme::glyphs::get("still"));
        append(&[TextChild::String(glyph.to_string())], style, now, out);
    } else if props.motion.as_deref() == Some(&TextMotion::Shimmer) {
        let mut drawn = super::Drawing::still(vec![Line::default()]);
        append(children, style, now, &mut drawn);
        super::motion::shimmer(&mut drawn.lines, now);
        if let (Some(last), Some(first)) = (out.lines.last_mut(), drawn.lines.first()) {
            last.spans.extend(first.spans.clone());
        }
        out.lines.extend(drawn.lines.into_iter().skip(1));
        out.next = out.next.into_iter().chain(drawn.next).min();
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
