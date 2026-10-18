//! Prompts interpret inline code and bold markers; prompts and documents share
//! the code rule so a look without a code ink keeps delimiters and surrounding ink.
use crate::theme;
use ratatui::{
    style::Style,
    text::{Line, Span},
};

pub fn line(text: &str, ink: Style) -> Line<'static> {
    let mut spans = Vec::new();
    let mut rest = text;
    while let Some((start, body, end, bold)) = marked(rest) {
        if start > 0 {
            spans.push(Span::styled(rest[..start].to_owned(), ink));
        }
        spans.push(if bold {
            Span::styled(body.to_owned(), ink.patch(theme::bold()))
        } else {
            code(body, ink)
        });
        rest = &rest[end..];
    }
    if !rest.is_empty() {
        spans.push(Span::styled(rest.to_owned(), ink));
    }
    Line::from(spans)
}

pub(crate) fn code(body: &str, ink: Style) -> Span<'static> {
    match theme::foreground("code").filter(|style| style.fg.is_some()) {
        Some(ink) => Span::styled(body.to_owned(), ink),
        None => Span::styled(format!("`{body}`"), ink),
    }
}

fn marked(text: &str) -> Option<(usize, &str, usize, bool)> {
    for (start, glyph) in text.char_indices() {
        let (mark, delimiter) = match glyph {
            '`' => ("`", '`'),
            '*' if text[start..].starts_with("**") => ("**", '*'),
            _ => continue,
        };
        let body = &text[start + mark.len()..];
        let Some(end) = body.find(delimiter) else {
            continue;
        };
        if end > 0 && body[end..].starts_with(mark) {
            return Some((
                start,
                &body[..end],
                start + mark.len() * 2 + end,
                glyph == '*',
            ));
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::style::Modifier;

    #[test]
    fn plain_code_keeps_delimiters_and_the_surrounding_ink() {
        theme::with(crate::painted::no_colour(), || {
            let ink = Style::default().add_modifier(Modifier::ITALIC);
            assert_eq!(code("cargo test", ink), Span::styled("`cargo test`", ink));
        });
    }

    #[test]
    fn coloured_code_uses_its_ink_without_delimiters() {
        assert_eq!(
            code("cargo test", theme::text()),
            Span::styled("cargo test", theme::foreground("code").unwrap())
        );
    }

    #[test]
    fn prompts_keep_block_markers_and_interpret_only_code_and_bold() {
        let drawn = line("# **bold** *literal* [link](url) `code`", Style::default());
        assert_eq!(drawn.to_string(), "# bold *literal* [link](url) code");
        assert!(
            drawn
                .spans
                .iter()
                .any(|span| span.content == "bold"
                    && span.style.add_modifier.contains(Modifier::BOLD))
        );
        for text in ["- item", "> quoted", "**open `open"] {
            assert_eq!(line(text, Style::default()).to_string(), text);
        }
    }
}
