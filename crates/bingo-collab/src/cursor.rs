//! Where a seat has read a room up to (ADR-0034 §2). A post is written once,
//! into the room's own journal, and what each seat has read of it is one fact
//! beside the posts: a `cursor:<member>` register in that same journal, a kind
//! per seat, so two seats reading at once write two facts rather than racing
//! over one (the `ear:` precedent, ADR-0029 §4). Everything that asks "has this
//! seat seen that?" — the reader at the head of a turn, the patience deadline,
//! the serial rule — asks the room, and nothing keeps a second answer.
//!
//! The room's journal is where it lives because a room is a `Log` session: it
//! answers nobody, so reading it costs nothing and takes nothing away. A
//! member's own session is not like that — a reader that opens one and lets go
//! is its last client, and closing it under an idle seat loses the very wake
//! this plugin just sent. So the cursor is kept where it can be read without
//! touching the seat at all, and a name nobody holds yet can be seated with one.
//!
//! ADR-0034 calls the cursor the room's `seq`. A `Seq` addresses a frame, and
//! the only door onto frames by seq is `events_since`, a stream that never ends
//! on its own — nothing a turn can drain. The kernel's own address for a post
//! is the `ItemId` its journal gave it, which a snapshot hands back in order,
//! so that is what the cursor holds: the same watermark, in the vocabulary the
//! host actually offers.

use bingo_sdk::{ItemId, SessionState};
use serde_json::{Value, json};

use crate::mentions::Post;
use crate::{PLUGIN, name};

/// The kind one seat's cursor is published under, before its name.
pub const CURSOR: &str = "cursor:";

/// The one thing a cursor payload holds.
const POST: &str = "post";

/// A room compares names in any case, so a register is keyed in one spelling
/// of it — the same rule an ear's register keeps.
pub fn kind(member: &str) -> String {
    format!("{CURSOR}{}", member.to_lowercase())
}

/// What a cursor is published as.
pub fn payload(post: &ItemId) -> Value {
    json!({ POST: post.as_str() })
}

/// The post a payload points at, or nothing for a payload that is not one.
pub fn stored(payload: &Value) -> Option<ItemId> {
    payload[POST].as_str().map(ItemId::from_raw)
}

/// Where one seat has read this room up to, as the room's own journal says.
/// A seat with no register has read none of it.
pub fn of_state(room: &SessionState, member: &str) -> Option<ItemId> {
    stored(room.extensions.get(PLUGIN)?.get(&kind(member))?)
}

/// The posts a seat has not read: the ones after its cursor, and all of them
/// for a seat that has none.
pub fn unread<'a>(posts: &'a [Post], cursor: Option<&ItemId>) -> &'a [Post] {
    let Some(read) = cursor else {
        return posts;
    };
    match posts.iter().position(|post| &post.id == read) {
        Some(at) => &posts[at + 1..],
        // A cursor the room does not hold points at nothing it can measure
        // against, so it says as little as no cursor at all.
        None => posts,
    }
}

/// What one seat has not read of one room: the posts it has to read, and the
/// head to move its cursor to once it has. A seat's own posts are in neither
/// ledger — it wrote them — but they do move the cursor, because a seat that
/// wrote the room's last word is not behind on it.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Unread {
    pub posts: Vec<Post>,
    pub head: Option<ItemId>,
}

impl Unread {
    /// What a seat has not read, from the room's own snapshot and nothing else.
    pub fn of(room: &SessionState, member: &str) -> Unread {
        let posts: Vec<Post> = room
            .transcript
            .items()
            .iter()
            .filter_map(|item| Post::of(item))
            .collect();
        let unread = unread(&posts, of_state(room, member).as_ref());
        Unread {
            head: unread.last().map(|post| post.id.clone()),
            posts: unread
                .iter()
                .filter(|post| !name::same(&post.author, member))
                .cloned()
                .collect(),
        }
    }

    pub fn is_empty(&self) -> bool {
        self.posts.is_empty()
    }
}


/// The persisted cursor payload.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct Cursor { pub post: ItemId }
