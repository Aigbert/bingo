//! Whose session the footer stands in away from home: a teammate's name, or
//! a room and who sits in it.
use super::ink;
use crate::{face::Faces, theme, tree::Tree, views::Drawing};
use bingo_sdk::SessionState;
use ratatui::text::{Line, Span};

/// Nothing at home; a teammate's name bold in its ink, or the room's.
pub fn draw(tree: &Tree) -> Option<Drawing> {
    let state = tree.viewing()?;
    let faces = Faces::of(tree);
    let name = Span::styled(
        crate::tree::name(state),
        ink(faces.ink(&state.summary.id)).patch(theme::bold()),
    );
    let mut spans = vec![name];
    if bingo_collab::room::is_room(&state.summary) {
        spans.extend(room(state, &faces));
    }
    Some(Drawing::still(vec![Line::from(spans)]))
}

fn room(state: &SessionState, faces: &Faces) -> Vec<Span<'static>> {
    if bingo_collab::room::closed_of(state) {
        return vec![
            Span::styled(theme::glyphs::get("sep"), ink("faint")),
            Span::styled("closed", ink("dim")),
        ];
    }
    let mut spans = vec![Span::raw(" ")];
    let seated = crate::seats::members(state);
    for member in seated
        .into_iter()
        .filter(|name| name != crate::seats::HOLDER)
    {
        spans.push(Span::raw(" "));
        let color = faces.who(&member);
        spans.push(Span::styled(member, ink(color)));
    }
    spans
}
