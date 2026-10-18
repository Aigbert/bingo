//! A small cell layout: measure children, distribute spare cells, then decorate.
use super::{Engine, Marks, fit, indent, marked_with, text};
use crate::theme;
use bingo_sdk::{
    View,
    view::{Align, BoxStyle, Direction, Justify, Wrap},
};
use ratatui::text::{Line, Span};

pub fn lines(
    children: &[View],
    style: &BoxStyle,
    width: usize,
    marks: &Marks,
    engine: &Engine<'_>,
) -> Vec<Line<'static>> {
    if style.display.as_deref() == Some("none") || width == 0 {
        return Vec::new();
    }
    let mx = space(style.spacing.margin_x.or(style.spacing.margin)).min(width / 2);
    let my = space(style.spacing.margin_y.or(style.spacing.margin));
    let available = width.saturating_sub(mx * 2);
    let width = if style.flex_grow.unwrap_or(0) > 0 {
        available
    } else {
        style
            .width
            .map_or(available, |size| usize::from(size).min(available))
    };
    let border = usize::from(
        style
            .border_style
            .as_deref()
            .is_some_and(|border| border != "none"),
    );
    let px = space(style.spacing.padding_x.or(style.spacing.padding)).min(width / 2);
    let py = space(style.spacing.padding_y.or(style.spacing.padding));
    let inner = width.saturating_sub(2 * (px + border));
    let children: Vec<_> = children.iter().filter(|child| !hidden(child)).collect();
    let height = style.height.map(|height| {
        usize::from(height)
            .min(1024)
            .saturating_sub(2 * (py + border))
    });
    let mut out = body(&children, style, inner, height, marks, engine);
    if let Some(height) = height {
        out.resize(height, Line::default());
        out.truncate(height);
    }
    out.splice(0..0, vec![Line::default(); py]);
    out.extend(vec![Line::default(); py]);
    let mut out: Vec<_> = out
        .into_iter()
        .map(|line| {
            fit(
                indent(fit(line, inner), px),
                width.saturating_sub(border * 2),
            )
        })
        .collect();
    if border > 0 {
        out = bordered(out, width, style);
    }
    margins(out, width, mx, my, style.ground.as_deref())
}

fn margins(
    lines: Vec<Line<'static>>,
    width: usize,
    mx: usize,
    my: usize,
    ground: Option<&str>,
) -> Vec<Line<'static>> {
    let mut out: Vec<_> = lines
        .into_iter()
        .map(|line| indent(text::fill(line, ground, width), mx))
        .collect();
    out.splice(0..0, vec![Line::default(); my]);
    out.extend(vec![Line::default(); my]);
    out
}
fn space(value: Option<u16>) -> usize {
    usize::from(value.unwrap_or(0)).min(128)
}
fn hidden(view: &View) -> bool {
    matches!(view,View::Box{layout,..} if layout.display.as_deref()==Some("none"))
}
fn hint(view: &View) -> usize {
    if let View::Box { layout, .. } = view
        && let Some(width) = layout.width
    {
        return usize::from(width);
    }
    view.fold()
        .lines()
        .map(unicode_width::UnicodeWidthStr::width)
        .max()
        .unwrap_or(0)
        .max(1)
}
fn grow(view: &View) -> usize {
    if let View::Box { layout, .. } = view {
        usize::from(layout.flex_grow.unwrap_or(0))
    } else {
        0
    }
}
fn body(
    children: &[&View],
    style: &BoxStyle,
    width: usize,
    height: Option<usize>,
    marks: &Marks,
    engine: &Engine<'_>,
) -> Vec<Line<'static>> {
    let gap = space(style.gap);
    let desired: usize = children
        .iter()
        .map(|child| hint(child))
        .sum::<usize>()
        .saturating_add(gap * children.len().saturating_sub(1));
    let is_row = style.flex_direction == Some(Direction::Row)
        && !(style.flex_wrap == Some(Wrap::Wrap) && desired > width);
    if is_row {
        return row(children, style, width, height, marks, engine);
    }
    let gap = if style.flex_direction == Some(Direction::Row) {
        0
    } else {
        gap
    };
    column(children, style, width, height, gap, marks, engine)
}
fn column(
    children: &[&View],
    style: &BoxStyle,
    width: usize,
    height: Option<usize>,
    gap: usize,
    marks: &Marks,
    engine: &Engine<'_>,
) -> Vec<Line<'static>> {
    let mut drawn: Vec<_> = children
        .iter()
        .map(|child| column_child(child, style, width, marks, engine))
        .collect();
    let used = drawn.iter().map(Vec::len).sum::<usize>() + gap * children.len().saturating_sub(1);
    let spare = height.unwrap_or(used).saturating_sub(used);
    let total: usize = children.iter().map(|child| grow(child)).sum();
    if total > 0 {
        for (lines, child) in drawn.iter_mut().zip(children) {
            let extra = (spare * grow(child)).checked_div(total).unwrap_or_default();
            if extra > 0 {
                let mut child = (*child).clone();
                if let View::Box { layout, .. } = &mut child {
                    layout.height = Some((lines.len() + extra).min(1024) as u16);
                }
                *lines = column_child(&child, style, width, marks, engine);
            }
        }
    }
    let used = drawn.iter().map(Vec::len).sum::<usize>() + gap * children.len().saturating_sub(1);
    let (before, extra) = justify(
        style.justify_content,
        height.unwrap_or(used).saturating_sub(used),
        children.len(),
    );
    let mut out = vec![Line::default(); before];
    for (index, lines) in drawn.into_iter().enumerate() {
        if index > 0 {
            out.extend(vec![Line::default(); gap + extra]);
        }
        out.extend(lines);
    }
    out
}
fn column_child(
    child: &View,
    style: &BoxStyle,
    width: usize,
    marks: &Marks,
    engine: &Engine<'_>,
) -> Vec<Line<'static>> {
    let child_width = if matches!(
        style.align_items,
        Some(Align::Center | Align::FlexEnd | Align::FlexStart)
    ) {
        hint(child).min(width)
    } else {
        width
    };
    let by = match style.align_items {
        Some(Align::Center) => width.saturating_sub(child_width) / 2,
        Some(Align::FlexEnd) => width.saturating_sub(child_width),
        _ => 0,
    };
    marked_with(child, child_width, marks, engine)
        .into_iter()
        .map(|line| indent(line, by))
        .collect()
}

fn row(
    children: &[&View],
    style: &BoxStyle,
    width: usize,
    height: Option<usize>,
    marks: &Marks,
    engine: &Engine<'_>,
) -> Vec<Line<'static>> {
    let gap = space(style.gap);
    let gaps = gap * children.len().saturating_sub(1);
    let mut widths: Vec<_> = children.iter().map(|child| hint(child)).collect();
    let spare = width.saturating_sub(widths.iter().sum::<usize>().saturating_add(gaps));
    let total: usize = children.iter().map(|child| grow(child)).sum();
    if total > 0 {
        for (size, child) in widths.iter_mut().zip(children) {
            *size += (spare * grow(child)).checked_div(total).unwrap_or_default();
        }
    }
    let used = widths.iter().sum::<usize>().saturating_add(gaps);
    let (before, extra) = justify(
        style.justify_content,
        width.saturating_sub(used),
        children.len(),
    );
    let drawn: Vec<_> = children
        .iter()
        .zip(&widths)
        .map(|(child, size)| marked_with(child, (*size).min(width), marks, engine))
        .collect();
    let height = height.unwrap_or_else(|| drawn.iter().map(Vec::len).max().unwrap_or(0));
    (0..height)
        .map(|y| {
            let mut spans = vec![Span::raw(" ".repeat(before))];
            for (index, lines) in drawn.iter().enumerate() {
                if index > 0 {
                    spans.push(Span::raw(" ".repeat(gap + extra)));
                }
                let slack = height.saturating_sub(lines.len());
                let top = match style.align_items {
                    Some(Align::Center) => slack / 2,
                    Some(Align::FlexEnd) => slack,
                    _ => 0,
                };
                let line = y
                    .checked_sub(top)
                    .and_then(|y| lines.get(y))
                    .cloned()
                    .unwrap_or_default();
                spans.extend(fit(line, widths[index].min(width)).spans);
            }
            fit(Line::from(spans), width)
        })
        .collect()
}
fn justify(justify: Option<Justify>, spare: usize, count: usize) -> (usize, usize) {
    match justify {
        Some(Justify::Center) => (spare / 2, 0),
        Some(Justify::FlexEnd) => (spare, 0),
        Some(Justify::SpaceBetween) if count > 1 => (0, spare / (count - 1)),
        Some(Justify::SpaceAround) if count > 0 => (spare / (count * 2), spare / count),
        Some(Justify::SpaceEvenly) => (spare / (count + 1), spare / (count + 1)),
        _ => (0, 0),
    }
}
fn bordered(lines: Vec<Line<'static>>, width: usize, style: &BoxStyle) -> Vec<Line<'static>> {
    let default = theme::border();
    let glyphs = if default.top_left == "+" {
        default
    } else {
        match style.border_style.as_deref() {
            Some("single") => ratatui::symbols::border::PLAIN,
            Some("double") => ratatui::symbols::border::DOUBLE,
            Some("bold") => ratatui::symbols::border::THICK,
            _ => default,
        }
    };
    let ink = text::color(theme::dim(), style.border_color.as_deref());
    let middle = width.saturating_sub(2);
    let edge = |left: &str, right: &str| {
        fit(
            Line::from(Span::styled(
                format!("{left}{}{right}", glyphs.horizontal_top.repeat(middle)),
                ink,
            )),
            width,
        )
    };
    let mut out = vec![edge(glyphs.top_left, glyphs.top_right)];
    for line in lines {
        let mut spans = vec![Span::styled(glyphs.vertical_left, ink)];
        spans.extend(fit(line, middle).spans);
        spans.push(Span::styled(glyphs.vertical_right, ink));
        out.push(fit(Line::from(spans), width));
    }
    out.push(edge(glyphs.bottom_left, glyphs.bottom_right));
    out
}
