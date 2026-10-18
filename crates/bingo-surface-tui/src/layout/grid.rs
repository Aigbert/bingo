//! Ember's shared cell grid and text measures; screens adopt it in M193–M200.
#![allow(dead_code)]
use crate::theme;
use ratatui::style::Style;
use ratatui::text::{Line, Span};
use unicode_width::UnicodeWidthStr;

pub const SIDE: usize = 0;
pub const TX: usize = 3;
pub const VX: usize = 5;
pub const AX: usize = 13;
pub const RM: usize = 2;
pub const MEASURE: usize = 100;

pub fn dock_cap(height: usize) -> usize {
    (height.saturating_mul(66) / 100).saturating_sub(2).max(6)
}

pub fn fit(line: Line<'static>, width: usize) -> Line<'static> {
    if width == 0 {
        return Line::default();
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
                return Line::from(out).style(line.style);
            }
            text.push(ch);
            used += cells;
        }
        out.push(Span::styled(text, span.style));
    }
    Line::from(out).style(line.style)
}

pub fn pad_to(line: Line<'static>, width: usize) -> Line<'static> {
    let mut line = fit(line, width);
    let pad = width.saturating_sub(line.width());
    if pad > 0 {
        line.spans.push(Span::raw(" ".repeat(pad)));
    }
    line
}

pub fn short_path(path: &str, width: usize, ink: Style) -> Line<'static> {
    let cut = path.rfind('/').map_or(0, |at| at + 1);
    let base = &path[cut..];
    let mut dirs: Vec<_> = path[..cut].split('/').filter(|s| !s.is_empty()).collect();
    let mut lead = if path.starts_with('/') {
        "/".to_string()
    } else {
        String::new()
    };
    let head = loop {
        let head = if dirs.is_empty() {
            lead.clone()
        } else {
            format!("{lead}{}/", dirs.join("/"))
        };
        if dirs.is_empty() || head.width() + base.width() <= width {
            break head;
        }
        dirs.remove(0);
        lead = format!("{}/", theme::glyphs::get("ell"));
    };
    fit(
        Line::from(vec![
            Span::styled(head, theme::dim()),
            Span::styled(base.to_string(), ink),
        ]),
        width,
    )
}

pub fn fmt_dur(ms: u64) -> String {
    let t = ms / 1000;
    match t {
        0..60 => format!("{t}s"),
        60..3600 => format!("{}m {:02}s", t / 60, t % 60),
        _ => format!("{}h {:02}m", t / 3600, t / 60 % 60),
    }
}

pub fn fmt_k(n: f64) -> String {
    if n < 1000.0 {
        return format!("{}", (n + 0.5).floor());
    }
    if n < 10000.0 {
        return format!("{:.1}k", n / 1000.0);
    }
    format!("{}k", (n / 1000.0 + 0.5).floor())
}

pub fn plural(n: usize, word: &str) -> String {
    format!("{n} {word}{}", if n == 1 { "" } else { "s" })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn grid_and_formatters_match_the_prototype() {
        assert_eq!([SIDE, TX, VX, AX, RM, MEASURE], [0, 3, 5, 13, 2, 100]);
        assert_eq!([dock_cap(0), dock_cap(24), dock_cap(32)], [6, 13, 19]);
        assert_eq!(fmt_dur(61999), "1m 01s");
        assert_eq!(fmt_dur(3660000), "1h 01m");
        assert_eq!(
            [fmt_k(999.0), fmt_k(1500.0), fmt_k(10500.0)],
            ["999", "1.5k", "11k"]
        );
        assert_eq!(plural(1, "step"), "1 step");
        assert_eq!(plural(2, "step"), "2 steps");
        assert_eq!(
            short_path("crates/core/session/queue.rs", 20, theme::text()).to_string(),
            "…/session/queue.rs"
        );
        let line = fit(Line::from("你好world"), 6);
        assert_eq!(line.to_string(), "你好w…");
        assert_eq!(line.spans.last().unwrap().style, theme::dim());
        assert_eq!(pad_to(Line::from("hi"), 4).to_string(), "hi  ");
    }
}
