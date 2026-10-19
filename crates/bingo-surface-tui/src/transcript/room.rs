//! A room's own stream (`theirPostLines`, `yourPostLines`, `typingLines`,
//! `roomHead`): its head, each post under its author, the roster changes the
//! room read where they happened, and whoever is writing an answer it is owed.
use super::Rows;
use super::post::{self, Voice};
use crate::{seats, theme, views::Drawing};
use bingo_collab::mentions::Post;
use bingo_collab::room::Move;
use bingo_sdk::Item;
use ratatui::text::{Line, Span};

/// What opens a room's stream: its name in the room's ink, and what it is for.
pub fn head(rows: &Rows<'_>) -> Drawing {
    let state = rows.state;
    let Some(title) = state.summary.title.as_deref() else {
        return Drawing::default();
    };
    let mut label = vec![Span::styled(
        title.to_owned(),
        crate::face::style(rows.faces.ink(&state.summary.id)).patch(theme::bold()),
    )];
    if let Some(purpose) = bingo_collab::room::purpose_of(state) {
        label.push(sep());
        label.push(Span::styled(purpose, theme::ash()));
    }
    let at = state.summary.created_at.as_millisecond();
    super::notice::said(Line::from(label), at, rows.width, rows.now)
}

/// One item of a room's stream: a roster change, or somebody's post.
pub fn post(item: &Item, place: &super::Place<'_>, rows: &Rows<'_>) -> Drawing {
    let Some(post) = Post::of(item) else {
        return Drawing::default();
    };
    let text = post.text.trim();
    if text.is_empty() {
        return Drawing::default();
    }
    let at = item.started_at.as_millisecond();
    if let Some(change) = Move::read(&post.author, text) {
        return moved(&change, at, rows);
    }
    let joined = joins(item, place.previous);
    if bingo_collab::name::same(&post.author, seats::HOLDER) {
        let open = bingo_collab::owed::of_state(rows.state);
        let owed = bingo_collab::owed::on(&open, &item.id);
        return post::yours(text, &owed, joined, rows);
    }
    let voice = Voice {
        name: &post.author,
        ink: rows.faces.who(&post.author),
    };
    post::theirs(&voice, text, at, joined, rows)
}

/// Whether a post follows another by its author, so it shares the name.
pub fn joins(item: &Item, previous: Option<&Item>) -> bool {
    previous
        .and_then(spoken)
        .zip(spoken(item))
        .is_some_and(|(above, below)| bingo_collab::name::same(&above, &below))
}

/// Who wrote a post that is words, not a roster change.
fn spoken(item: &Item) -> Option<String> {
    let post = Post::of(item)?;
    Move::read(&post.author, post.text.trim())
        .is_none()
        .then_some(post.author)
}

/// A roster change, read where it happened: one notice per name that came or
/// went, or the room's end and why.
fn moved(change: &Move, at: i64, rows: &Rows<'_>) -> Drawing {
    let labels: Vec<Line<'static>> = match change {
        Move::Seated(names) => names
            .iter()
            .map(|name| came(name, " joined", rows))
            .collect(),
        Move::Unseated(names) => names.iter().map(|name| came(name, " left", rows)).collect(),
        Move::Closed { why, .. } => vec![closed(why.as_deref())],
    };
    let mut out = Drawing::default();
    for label in labels {
        out.append(super::notice::said(label, at, rows.width, rows.now));
    }
    out
}

fn came(name: &str, word: &'static str, rows: &Rows<'_>) -> Line<'static> {
    Line::from(vec![
        Span::styled(
            name.to_owned(),
            crate::face::style(rows.faces.who(name)).patch(theme::bold()),
        ),
        Span::styled(word, theme::ash()),
    ])
}

fn closed(why: Option<&str>) -> Line<'static> {
    let mut label = vec![Span::styled("closed", theme::ash())];
    if let Some(why) = why {
        label.push(sep());
        label.push(Span::styled(why.to_owned(), theme::text()));
    }
    Line::from(label)
}

fn sep() -> Span<'static> {
    Span::styled(
        theme::glyphs::get("sep"),
        theme::foreground("faint").unwrap_or_default(),
    )
}

/// Who is writing an answer the room is owed, a blank row above: a member
/// called on, still owing, whose own session runs a turn.
pub fn writers(rows: &Rows<'_>) -> Drawing {
    let mut out = Drawing::default();
    let mut named: Vec<String> = Vec::new();
    for mention in bingo_collab::owed::of_state(rows.state) {
        let Some(name) = mention.owed_by.chased() else {
            continue;
        };
        if named
            .iter()
            .any(|held| bingo_collab::name::same(held, name))
            || !rows.faces.works(name)
        {
            continue;
        }
        named.push(name.to_owned());
        let voice = Voice {
            name,
            ink: rows.faces.who(name),
        };
        if out.lines.is_empty() {
            out.lines.push(Line::default());
        }
        out.append(post::writing(&voice, rows.now));
    }
    out
}
