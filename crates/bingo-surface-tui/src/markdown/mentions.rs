//! Known names colour prose without touching code, URLs or unknown words.
use crate::{
    face, theme,
    tree::{self, Tree},
};
use ratatui::{style::Style, text::Span};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Name {
    pub text: String,
    pub room: bool,
    pub ink: &'static str,
}

pub fn known(tree: &Tree, faces: &crate::face::Faces) -> Vec<Name> {
    tree.sessions()
        .map(|state| Name {
            text: tree::name(state),
            room: bingo_collab::room::is_room(&state.summary),
            ink: faces.ink(&state.summary.id),
        })
        .chain([Name {
            text: crate::seats::HOLDER.into(),
            room: false,
            ink: faces.who(crate::seats::HOLDER),
        }])
        .collect()
}

pub fn spans(text: &str, style: Style, names: &[Name]) -> Vec<Span<'static>> {
    let mut out = Vec::new();
    let mut plain = 0;
    for (at, mark) in text.char_indices() {
        if !matches!(mark, '@' | '#') || !boundary(&text[..at]) {
            continue;
        }
        let end = text[at + 1..]
            .char_indices()
            .find(|(_, c)| !in_name(*c))
            .map(|(end, _)| at + 1 + end)
            .unwrap_or(text.len());
        let token = &text[at..end];
        let Some(name) = names.iter().find(|name| matches(name, token)) else {
            continue;
        };
        if plain < at {
            out.push(Span::styled(text[plain..at].to_string(), style));
        }
        let ink = if name.room {
            theme::mode()
        } else {
            face::style(name.ink)
        };
        out.push(Span::styled(token.to_string(), style.patch(ink)));
        plain = end;
    }
    if plain < text.len() {
        out.push(Span::styled(text[plain..].to_string(), style));
    }
    out
}

fn in_name(c: char) -> bool {
    c.is_alphanumeric() || matches!(c, '_' | '-')
}

fn boundary(before: &str) -> bool {
    before
        .chars()
        .next_back()
        .is_none_or(|c| !in_name(c) && !matches!(c, '/' | '@' | '#'))
}

fn matches(name: &Name, token: &str) -> bool {
    let (mark, word) = token.split_at(1);
    if name.room {
        mark == "#" && bingo_collab::name::same(&name.text, token)
    } else {
        mark == "@" && bingo_collab::name::same(&name.text, word)
    }
}
