//! Styling does not divide a word; punctuation travels with its neighbour.
use ratatui::text::{Line, Span};

pub(super) enum Unit {
    Space(Span<'static>),
    Word(Vec<Span<'static>>),
}

const NO_START: &str = "，。、；：！？）」』】》〉〕］｝”’,.;:!?)]}%";
const NO_END: &str = "（「『【《〈〔［｛“‘([{";

pub(super) fn units(line: &Line<'static>) -> Vec<Unit> {
    let mut out = Vec::new();
    let mut prev: Option<&str> = None;
    for span in &line.spans {
        for token in tokens(&span.content) {
            let part = Span::styled(token.to_string(), span.style);
            if token.starts_with([' ', '\t', '\n']) {
                out.push(Unit::Space(part));
                prev = None;
                continue;
            }
            let glue = prev.is_some_and(|prev| {
                token.starts_with(|c| NO_START.contains(c))
                    || prev.ends_with(|c| NO_END.contains(c))
                    || (!one_cjk(token) && !one_cjk(prev))
            });
            if glue && let Some(Unit::Word(parts)) = out.last_mut() {
                parts.push(part);
            } else {
                out.push(Unit::Word(vec![part]));
            }
            prev = Some(token);
        }
    }
    out
}

fn one_cjk(text: &str) -> bool {
    let mut chars = text.chars();
    chars.next().is_some_and(cjk) && chars.next().is_none()
}

fn cjk(c: char) -> bool {
    matches!(c as u32, 0x1100..=0x115f | 0x2e80..=0xa4cf | 0xac00..=0xd7a3 |
        0xf900..=0xfaff | 0xfe30..=0xfe4f | 0xff00..=0xff60 | 0xffe0..=0xffe6)
}

fn tokens(text: &str) -> Vec<&str> {
    let mut out = Vec::new();
    let mut chars = text.char_indices().peekable();
    while let Some((start, c)) = chars.next() {
        if text[start..].starts_with("[image ")
            && let Some(close) = text[start..].find(']')
        {
            let end = start + close + 1;
            while chars.peek().is_some_and(|(at, _)| *at < end) {
                chars.next();
            }
            out.push(&text[start..end]);
            continue;
        }
        let space = matches!(c, ' ' | '\t');
        let mut end = start + c.len_utf8();
        if c != '\n' && !cjk(c) {
            while let Some(&(at, next)) = chars.peek() {
                if next == '\n' || cjk(next) || matches!(next, ' ' | '\t') != space {
                    break;
                }
                end = at + next.len_utf8();
                chars.next();
            }
        }
        out.push(&text[start..end]);
    }
    out
}
