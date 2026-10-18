//! Word wrapping for styled lines. The transcript is measured, not predicted:
//! every view that needs to know how tall a block is wraps it first and counts.
use ratatui::text::{Line, Span};
use unicode_segmentation::UnicodeSegmentation;
use unicode_width::UnicodeWidthStr;
mod units;
use units::{Unit, units};

/// Narrow a region to the person's `tui.measure`. Prose applies its own
/// 100-cell cap inside that region (ADR-0097 §3).
pub fn measure(width: usize, cap: Option<usize>) -> usize {
    cap.map_or(width, |cap| width.min(cap))
}

/// Wrap one styled line to `width` columns, keeping each span's style. An empty
/// line stays one empty line, so blank separators survive.
pub fn wrap(line: &Line<'static>, width: usize) -> Vec<Line<'static>> {
    if width < 2 {
        return vec![line.clone()];
    }
    let mut rows = Rows {
        out: Vec::new(),
        current: Vec::new(),
        used: 0,
        soft: false,
        width,
    };
    for unit in units(line) {
        rows.place(unit);
    }
    trim_end(&mut rows.current);
    rows.out.push(Line::from(rows.current));
    rows.out
        .into_iter()
        .map(|row| row.style(line.style))
        .collect()
}

/// Wrap every line of a block and flatten the result.
pub fn wrap_all(lines: &[Line<'static>], width: usize) -> Vec<Line<'static>> {
    lines.iter().flat_map(|line| wrap(line, width)).collect()
}

struct Rows {
    out: Vec<Line<'static>>,
    current: Vec<Span<'static>>,
    used: usize,
    soft: bool,
    width: usize,
}

impl Rows {
    fn place(&mut self, unit: Unit) {
        match unit {
            Unit::Space(span) => self.space(span),
            Unit::Word(parts) => {
                let width: usize = parts.iter().map(Span::width).sum();
                if width > self.width.saturating_sub(self.used)
                    && self.used > 0
                    && width <= self.width
                {
                    self.brk(true);
                }
                for part in parts {
                    self.cut(part);
                }
            }
        }
    }

    fn space(&mut self, span: Span<'static>) {
        if span.content == "\n" {
            self.brk(false);
            return;
        }
        if self.used == 0 && self.soft {
            return;
        }
        if self.used + span.width() > self.width {
            self.brk(true);
            return;
        }
        self.push(span);
    }

    fn cut(&mut self, span: Span<'static>) {
        let mut text = String::new();
        for grapheme in span.content.graphemes(true) {
            if self.used + text.width() + grapheme.width() > self.width {
                if !text.is_empty() {
                    self.push(Span::styled(std::mem::take(&mut text), span.style));
                }
                if self.used > 0 {
                    self.brk(true);
                }
            }
            text.push_str(grapheme);
        }
        if !text.is_empty() {
            self.push(Span::styled(text, span.style));
        }
    }

    fn push(&mut self, span: Span<'static>) {
        self.used += span.width();
        self.current.push(span);
    }

    fn brk(&mut self, soft: bool) {
        trim_end(&mut self.current);
        self.out.push(Line::from(std::mem::take(&mut self.current)));
        self.used = 0;
        self.soft = soft;
    }
}

fn trim_end(spans: &mut Vec<Span<'static>>) {
    while let Some(last) = spans.last_mut() {
        last.content = last
            .content
            .trim_end_matches([' ', '\t'])
            .to_string()
            .into();
        if !last.content.is_empty() {
            break;
        }
        spans.pop();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::theme;

    fn text(lines: &[Line<'static>]) -> Vec<String> {
        lines.iter().map(|l| l.to_string()).collect()
    }

    /// The transcript fills what it is given until a person says otherwise
    /// (design §7, M89).
    #[test]
    fn a_measure_is_the_region_until_a_person_asks_for_less() {
        assert_eq!(measure(200, None), 200);
        assert_eq!(measure(200, Some(100)), 100);
        assert_eq!(measure(80, Some(100)), 80, "and never more than the region");
    }

    #[test]
    fn words_break_at_spaces_and_drop_the_space_at_the_break() {
        let line = Line::from("the quick brown fox jumps");
        assert_eq!(
            text(&wrap(&line, 10)),
            vec!["the quick", "brown fox", "jumps"]
        );
    }

    #[test]
    fn a_word_wider_than_the_line_is_cut() {
        let line = Line::from("abcdefghijkl");
        assert_eq!(text(&wrap(&line, 5)), vec!["abcde", "fghij", "kl"]);
    }

    /// Prose with no spaces in it — Chinese, a long path — starts beside the
    /// mark that introduces it rather than under it on a line of its own.
    #[test]
    fn a_word_wider_than_the_line_fills_the_line_it_starts_on() {
        let line = Line::from(vec![Span::raw("ab "), Span::raw("cdefghij")]);
        assert_eq!(text(&wrap(&line, 5)), vec!["ab cd", "efghi", "j"]);
        let line = Line::from(vec![
            Span::raw("▍ 备注: "),
            Span::raw("已经帮你查看了记忆索引"),
        ]);
        assert_eq!(
            text(&wrap(&line, 14)),
            vec!["▍ 备注: 已经帮", "你查看了记忆索", "引"]
        );
    }

    #[test]
    fn styles_survive_the_break() {
        let line = Line::from(vec![
            Span::styled("hello ".to_string(), theme::bold()),
            Span::styled("world".to_string(), theme::dim()),
        ]);
        let wrapped = wrap(&line, 6);
        assert_eq!(text(&wrapped), vec!["hello", "world"]);
        assert_eq!(wrapped[0].spans[0].style, theme::bold());
        assert_eq!(wrapped[1].spans[0].style, theme::dim());
    }

    #[test]
    fn an_empty_line_stays_one_line() {
        assert_eq!(wrap(&Line::from(""), 10).len(), 1);
    }

    #[test]
    fn wide_glyphs_count_two_columns() {
        let line = Line::from("你好世界");
        assert_eq!(text(&wrap(&line, 4)), vec!["你好", "世界"]);
    }
}

#[cfg(test)]
mod prototype_tests;
