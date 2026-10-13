//! The pure serial checkpoint ledgers.
use crate::name as names;
use bingo_sdk::{ContentPart, Item, ItemBody, ItemId, SessionState};
/// One post, as a room's journal has it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Post {
    pub author: String,
    pub text: String,
}

/// What the room says about a post about to be written.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Verdict {
    /// Everything the caller could have read, it has read.
    Land,
    /// These landed since, in order, and the post was not written against
    /// them.
    Behind { seen: usize, missed: Vec<Post> },
}

/// The posts of a room the caller was there to hear, each with where the room
/// holds it: everyone else's, from the moment its own session opened.
pub fn awaited(room: &SessionState, caller: &SessionState, speaker: &str) -> Vec<(usize, Post)> {
    room.transcript
        .items()
        .iter()
        .enumerate()
        .filter(|(_, item)| {
            item.completed_at.unwrap_or(item.started_at) >= caller.summary.created_at
        })
        .filter_map(|(at, item)| Some((at, post_of(item)?)))
        .filter(|(_, post)| post.author != speaker)
        .collect()
}

/// What a room holds is what was said into it: a user item, and nothing else.
fn post_of(item: &Item) -> Option<Post> {
    let ItemBody::User { parts, origin } = &item.body else {
        return None;
    };
    Some(Post {
        author: origin
            .principal
            .clone()
            .unwrap_or_else(|| names::PARENT.to_string()),
        text: parts.iter().filter_map(ContentPart::as_text).collect(),
    })
}

/// How far the caller's cursor stands: the posts it awaited that the room
/// holds at or before the one the cursor names (ADR-0034 §5). A seat with no
/// cursor, or one the room does not hold, has read nothing.
pub fn read(room: &SessionState, awaited: &[(usize, Post)], cursor: Option<ItemId>) -> usize {
    let Some(head) = cursor.and_then(|id| {
        room.transcript
            .items()
            .iter()
            .position(|item| item.id == id)
    }) else {
        return 0;
    };
    awaited.iter().filter(|(at, _)| *at <= head).count()
}

pub fn verdict(awaited: Vec<(usize, Post)>, seen: usize) -> Verdict {
    if seen >= awaited.len() {
        return Verdict::Land;
    }
    Verdict::Behind {
        seen,
        missed: awaited[seen..]
            .iter()
            .map(|(_, post)| post.clone())
            .collect(),
    }
}
