//! One hunk parser and replacement pass, with the frame supplied by its caller.
use crate::theme;
use ratatui::style::Style;
use ratatui::text::{Line, Span};
use std::ops::Range;

/// How a caller frames the rows: a well numbers them and tints their ground,
/// the permission card keeps the headers and inks the signs alone.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum DiffStyle {
    Well,
    #[default]
    Permission,
}

impl DiffStyle {
    fn gutter(self) -> bool {
        self == Self::Well
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Class {
    Header,
    Context,
    Removed,
    Added,
}

struct Row<'a> {
    number: u32,
    class: Class,
    text: &'a str,
}

fn rows(unified: &str) -> Vec<Row<'_>> {
    let (mut old, mut new) = (0u32, 0u32);
    unified
        .lines()
        .map(|line| {
            let class = if line.starts_with("@@") {
                let mut parts = line.split_whitespace().skip(1);
                old = coordinate(parts.next());
                new = coordinate(parts.next());
                Class::Header
            } else if line.starts_with("---") || line.starts_with("+++") {
                Class::Header
            } else {
                match line.as_bytes().first() {
                    Some(b'-') => Class::Removed,
                    Some(b'+') => Class::Added,
                    Some(b' ') => Class::Context,
                    _ => Class::Header,
                }
            };
            let number = if class == Class::Removed { old } else { new };
            if matches!(class, Class::Removed | Class::Context) {
                old = old.saturating_add(1);
            }
            if matches!(class, Class::Added | Class::Context) {
                new = new.saturating_add(1);
            }
            Row {
                number,
                class,
                text: if class == Class::Header {
                    line
                } else {
                    &line[1..]
                },
            }
        })
        .collect()
}

fn coordinate(part: Option<&str>) -> u32 {
    part.and_then(|part| part.get(1..))
        .and_then(|part| part.split(',').next())
        .and_then(|part| part.parse().ok())
        .unwrap_or(0)
}

fn word_spans(a: &str, b: &str) -> (Range<usize>, Range<usize>) {
    let prefix = a
        .chars()
        .zip(b.chars())
        .take_while(|(a, b)| a == b)
        .map(|(a, _)| a.len_utf8())
        .sum::<usize>();
    let suffix = a[prefix..]
        .chars()
        .rev()
        .zip(b[prefix..].chars().rev())
        .take_while(|(a, b)| a == b)
        .map(|(a, _)| a.len_utf8())
        .sum::<usize>();
    (prefix..a.len() - suffix, prefix..b.len() - suffix)
}

fn replacements(rows: &[Row<'_>]) -> Vec<Option<Range<usize>>> {
    let mut spans = vec![None; rows.len()];
    let mut at = 0;
    while at < rows.len() {
        let removed = rows[at..]
            .iter()
            .take_while(|row| row.class == Class::Removed)
            .count();
        let added = rows[at + removed..]
            .iter()
            .take_while(|row| row.class == Class::Added)
            .count();
        for n in 0..removed.min(added) {
            let (before, after) = word_spans(rows[at + n].text, rows[at + removed + n].text);
            spans[at + n] = Some(before);
            spans[at + removed + n] = Some(after);
        }
        at += (removed + added).max(1);
    }
    spans
}

pub fn lines(unified: &str, style: DiffStyle) -> Vec<Line<'static>> {
    let rows = rows(unified);
    rows.iter()
        .zip(replacements(&rows))
        .filter(|(row, _)| !(row.class == Class::Header && style.gutter()))
        .map(|(row, changed)| line(row, changed, style))
        .collect()
}

fn inks(class: Class, style: DiffStyle) -> (Style, Style, Style) {
    let (ink, ground, word) = match class {
        Class::Added => (theme::added(), "addTint", "addWord"),
        Class::Removed => (theme::removed(), "delTint", "delWord"),
        _ => (theme::dim(), "well", "well"),
    };
    match style {
        DiffStyle::Permission => (ink, Style::default(), ink.patch(theme::bold())),
        DiffStyle::Well => {
            let ink = if class == Class::Context {
                theme::ash()
            } else {
                theme::text()
            };
            (
                ink,
                theme::ground(ground).unwrap_or_default(),
                ink.patch(theme::ground(word).unwrap_or_default()),
            )
        }
    }
}

fn line(row: &Row<'_>, changed: Option<Range<usize>>, style: DiffStyle) -> Line<'static> {
    let (ink, ground, emphasis) = inks(row.class, style);
    let (sign, sign_ink) = match row.class {
        Class::Added => ("+", theme::good()),
        Class::Removed => (
            if matches!(style, DiffStyle::Well) {
                theme::glyphs::get("minus")
            } else {
                "-"
            },
            theme::bad(),
        ),
        Class::Context => (" ", ink),
        Class::Header => ("", ink),
    };
    let mut spans = Vec::new();
    if style.gutter() {
        spans.push(Span::styled(format!("{:>4} ", row.number), theme::dim()));
    }
    spans.push(Span::styled(
        sign,
        if matches!(style, DiffStyle::Well) {
            sign_ink
        } else {
            ink
        },
    ));
    if matches!(style, DiffStyle::Well) {
        spans.push(Span::raw(" "));
    }
    if let Some(range) = changed.filter(|range| !range.is_empty()) {
        spans.push(Span::styled(row.text[..range.start].to_owned(), ink));
        spans.push(Span::styled(row.text[range.clone()].to_owned(), emphasis));
        spans.push(Span::styled(row.text[range.end..].to_owned(), ink));
    } else {
        spans.push(Span::styled(row.text.to_owned(), ink));
    }
    let mut joined = Vec::<Span<'static>>::new();
    for span in spans {
        if let Some(last) = joined.last_mut()
            && last.style == span.style
        {
            last.content.to_mut().push_str(&span.content);
        } else {
            joined.push(span);
        }
    }
    Line::from(joined).style(ground)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn line_numbers_follow_both_hunks_without_headers() {
        let lines = lines(
            "--- a/x\n+++ b/x\n@@ -10,2 +20,2 @@\n same\n-old\n+new\n@@ -40 +50 @@\n-gone\n+here",
            DiffStyle::Well,
        );
        let numbers: Vec<_> = lines
            .iter()
            .map(|line| line.spans[0].content.trim().to_string())
            .collect();
        assert_eq!(numbers, ["20", "11", "21", "40", "50"]);
    }
    #[test]
    fn replacement_runs_pair_in_order_and_keep_unpaired_rows_plain() {
        let parsed = rows("@@ -10,3 +20,2 @@\n-a old z\n-b old z\n-unpaired\n+a new z\n+b new z");
        let changes = replacements(&parsed);
        assert_eq!(
            changes,
            vec![None, Some(2..5), Some(2..5), None, Some(2..5), Some(2..5)]
        );
    }
    #[test]
    fn word_spans_keep_shared_ends_and_utf8_boundaries() {
        assert_eq!(word_spans("foo(old)", "foo(new)"), (4..7, 4..7));
        assert_eq!(word_spans("你好旧尾", "你好新尾"), (6..9, 6..9));
        assert_eq!(word_spans("same", "same"), (4..4, 4..4));
        assert_eq!(word_spans("", "added"), (0..0, 0..5));
    }
}
