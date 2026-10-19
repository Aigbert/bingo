//! Ember's shared cell grid and text measures; screens adopt it in M193–M200.
use crate::theme;
use ratatui::text::{Line, Span};
use unicode_width::UnicodeWidthStr;

pub const TX: usize = 3;
pub const VX: usize = 5;
pub const AX: usize = 13;
pub const RM: usize = 2;
pub const MEASURE: usize = 100;

pub fn dock_cap(height: usize) -> u16 {
    u16::try_from((height.saturating_mul(66) / 100).saturating_sub(2).max(6)).unwrap_or(u16::MAX)
}

pub fn fit(line: Line<'static>, width: usize) -> Line<'static> {
    if width == 0 {
        return Line {
            alignment: line.alignment,
            ..Line::default()
        };
    }
    if line.width() <= width {
        return line;
    }
    let ell = theme::glyphs::get("ell");
    let room = width.saturating_sub(ell.width());
    let mut used = 0;
    let mut out = Vec::new();
    for span in line.spans {
        let mut text = String::new();
        for ch in span.content.chars() {
            let cells = unicode_width::UnicodeWidthChar::width(ch).unwrap_or(0);
            if used + cells > room {
                out.push(Span::styled(
                    text.trim_end_matches(' ').to_string(),
                    span.style,
                ));
                out.push(Span::styled(ell, theme::dim()));
                return Line {
                    style: line.style,
                    alignment: line.alignment,
                    ..Line::from(out)
                };
            }
            text.push(ch);
            used += cells;
        }
        out.push(Span::styled(text, span.style));
    }
    Line {
        style: line.style,
        alignment: line.alignment,
        ..Line::from(out)
    }
}

pub fn pad_to(line: Line<'static>, width: usize) -> Line<'static> {
    let mut line = fit(line, width);
    let pad = width.saturating_sub(line.width());
    if pad > 0 {
        line.spans.push(Span::raw(" ".repeat(pad)));
    }
    line
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn grid_and_fit_match_the_prototype() {
        assert_eq!([TX, VX, AX, RM, MEASURE], [3, 5, 13, 2, 100]);
        assert_eq!([dock_cap(0), dock_cap(24), dock_cap(32)], [6, 13, 19]);
        assert_eq!(
            fit(Line::from("long text").centered(), 4).alignment,
            Some(ratatui::layout::Alignment::Center)
        );
        let line = fit(Line::from("你好world"), 6);
        assert_eq!(line.to_string(), "你好w…");
        assert_eq!(line.spans.last().unwrap().style, theme::dim());
        assert_eq!(pad_to(Line::from("hi"), 4).to_string(), "hi  ");
    }
}
