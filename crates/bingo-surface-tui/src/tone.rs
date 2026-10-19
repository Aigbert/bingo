//! Parts keep their own inks in both a footer toast and a transcript headline.
use crate::{clock::Now, theme, views};
use bingo_sdk::{Tone, view::TextMotion};
use pulldown_cmark::{Event, Parser, Tag, TagEnd};
use ratatui::{style::Style, text::Span};

pub fn parts(
    parts: &[String],
    tone: Tone,
    markdown: bool,
    fade: Option<(TextMotion, Now)>,
) -> Vec<Span<'static>> {
    let mut out = Vec::new();
    let mark = match tone {
        Tone::Good => Some(("ok", "good")),
        Tone::Bad => Some(("times", "bad")),
        _ => None,
    };
    if let Some((mark, ink)) = mark {
        out.push(span(format!("{} ", theme::glyphs::get(mark)), ink, &fade));
    }
    for (n, part) in parts.iter().enumerate() {
        if n > 0 {
            out.push(span(theme::glyphs::get("sep").into(), "faint", &fade));
        }
        let ink = if n == 0 && tone == Tone::Attention {
            "amber"
        } else {
            "muted"
        };
        if markdown {
            out.extend(inline(part, ink, &fade));
        } else {
            out.push(span(part.clone(), ink, &fade));
        }
    }
    out
}

fn style(ink: &str, fade: &Option<(TextMotion, Now)>) -> Style {
    let style = theme::foreground(ink).unwrap_or_default();
    let Some((motion, now)) = fade else {
        return style;
    };
    let base = theme::palette().named(ink).unwrap_or(theme::palette().fg());
    views::motion::style(style, base, Some(motion), Some(*now))
}

fn span(text: String, ink: &str, fade: &Option<(TextMotion, Now)>) -> Span<'static> {
    Span::styled(text, style(ink, fade))
}

fn inline(text: &str, ink: &str, fade: &Option<(TextMotion, Now)>) -> Vec<Span<'static>> {
    let mut out = Vec::new();
    let mut bold = false;
    let mut italic = false;
    for event in Parser::new(text) {
        let mut part = match event {
            Event::Text(text) => span(text.into_string(), ink, fade),
            Event::Code(text) => {
                let mut part = crate::markdown::inline_code(&text, style(ink, fade));
                if theme::foreground("code").is_some_and(|s| s.fg.is_some()) {
                    part.style = style("code", fade);
                }
                part
            }
            Event::Start(Tag::Strong) => {
                bold = true;
                continue;
            }
            Event::End(TagEnd::Strong) => {
                bold = false;
                continue;
            }
            Event::Start(Tag::Emphasis) => {
                italic = true;
                continue;
            }
            Event::End(TagEnd::Emphasis) => {
                italic = false;
                continue;
            }
            Event::SoftBreak | Event::HardBreak => span(" ".into(), ink, fade),
            _ => continue,
        };
        if bold {
            part.style = part.style.patch(theme::bold());
        }
        if italic {
            part.style = part.style.patch(theme::italic());
        }
        out.push(part);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn attention_inks_only_its_first_part_and_plugin_text_stays_whole() {
        let drawn = parts(
            &["cannot read".into(), "alt+m switches model".into()],
            Tone::Attention,
            false,
            None,
        );
        assert_eq!(drawn[0].style, theme::foreground("amber").unwrap());
        assert_eq!(drawn[1].style, theme::foreground("faint").unwrap());
        assert_eq!(drawn[2].style, theme::foreground("muted").unwrap());
        let one = parts(&["a · b".into()], Tone::Neutral, false, None);
        assert_eq!(one.len(), 1);
    }
    #[test]
    fn headlined_code_preserves_embedded_backticks_and_plain_escapes() {
        let drawn = parts(
            &["``echo `date` `` finished".into(), r"\*ok\_".into()],
            Tone::Good,
            true,
            None,
        );
        assert!(
            drawn
                .iter()
                .any(|s| s.content == "echo `date` "
                    && s.style == theme::foreground("code").unwrap())
        );
        assert!(
            drawn
                .iter()
                .map(|s| s.content.as_ref())
                .collect::<String>()
                .ends_with("*ok_")
        );
    }
}
