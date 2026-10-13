//! What a room's posts owe (ADR-0022 §1–2). `@name` opens a debt against that
//! member and the member's next post closes it — speaking is the answer, and
//! what was said is deliberately not judged. `@all` is one debt against the
//! room, closed by any other member.
//!
//! All of it is a fold of the room's own journal: the questions are already
//! there and so are the answers, so there is nothing to store beside them and
//! the next process re-derives exactly what this one had.

use bingo_sdk::{ContentPart, Item, ItemBody, ItemId, SessionState};
use jiff::Timestamp;

use crate::name::{PARENT, same};
use crate::room;

/// The sigil that calls on the room rather than on anyone in it — which is
/// every member of it but whoever posted. A room that seats a member of this
/// name spends the word on the member: a real name is never shadowed.
const ALL: &str = "all";

/// Characters of a post that a nudge quotes back. Enough to tell two
/// questions apart, short enough for one line.
pub const HEAD: usize = 48;

/// Who a debt is against.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Owed {
    /// A named member: the one a nudge goes to.
    Member(String),
    /// `@all`: the room. Everyone in it is called on and the fan-out reads it
    /// so, but the sigil picked no member, so the chase picks none either.
    Room,
}

impl Owed {
    /// The member to chase, or nothing at all.
    pub fn chased(&self) -> Option<&str> {
        match self {
            Owed::Member(name) => Some(name),
            Owed::Room => None,
        }
    }

    /// How it reads in `/room` and on the card.
    pub fn said(&self) -> String {
        match self {
            Owed::Member(name) => name.clone(),
            Owed::Room => format!("@{ALL}"),
        }
    }
}

/// One `@` nobody has answered yet.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Mention {
    pub owed_by: Owed,
    /// Who asked, as the post signed itself.
    pub asker: String,
    /// The post that opened it: what tells two debts apart, and what a nudge
    /// quotes.
    pub post: ItemId,
    pub at: Timestamp,
    pub head: String,
}

/// A post, as the fold reads one.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Post {
    pub id: ItemId,
    /// A post nobody signed came from the session the room hangs under.
    pub author: String,
    pub at: Timestamp,
    pub text: String,
}

impl Post {
    /// The post a journal item is, or nothing for an item that is not one:
    /// what a room holds is what was said into it.
    pub fn of(item: &Item) -> Option<Post> {
        let ItemBody::User { parts, origin } = &item.body else {
            return None;
        };
        Some(Post {
            id: item.id.clone(),
            author: origin
                .principal
                .clone()
                .unwrap_or_else(|| PARENT.to_string()),
            at: item.completed_at.unwrap_or(item.started_at),
            text: parts.iter().filter_map(ContentPart::as_text).collect(),
        })
    }
}

/// Every debt still open after the last post, in the order they were opened.
/// Settle before open, so a post that answers and asks in one breath — `thanks
/// @scout — @reviewer what about x?` — closes its author's own debt and opens
/// the reviewer's.
pub fn mentions(posts: &[Post], members: &[String]) -> Vec<Mention> {
    let mut open: Vec<Mention> = Vec::new();
    for post in posts {
        settle(&mut open, &post.author, members);
        open.extend(opened(post, members));
    }
    open
}

/// What a post's author no longer owes. Their own debts close because they
/// spoke; the room's `@all` closes for any other member, and never for the one
/// who asked it.
fn settle(open: &mut Vec<Mention>, author: &str, members: &[String]) {
    open.retain(|mention| match &mention.owed_by {
        Owed::Member(name) => !same(name, author),
        Owed::Room => same(&mention.asker, author) || !is_member(author, members),
    });
}

/// The debts a post opens: every member it calls on but its own author — a
/// question to yourself is not one — and one against the room for `@all`.
fn opened(post: &Post, members: &[String]) -> Vec<Mention> {
    named(&post.text, members)
        .into_iter()
        .filter(|owed| owed.chased().is_none_or(|name| !same(name, &post.author)))
        .map(|owed| Mention {
            owed_by: owed,
            asker: post.author.clone(),
            post: post.id.clone(),
            at: post.at,
            head: head(&post.text),
        })
        .collect()
}

/// Who a post calls on: `@name` at a word boundary, matched case-insensitively
/// against the room's members. `mail@user` is an address rather than a call,
/// and a name nobody in the room has asks nothing of anybody. The delivery asks
/// this too (ADR-0029 §5) — one matcher, against the one roster.
pub fn named(text: &str, members: &[String]) -> Vec<Owed> {
    let chars: Vec<char> = text.chars().collect();
    let mut found: Vec<Owed> = Vec::new();
    for (i, c) in chars.iter().enumerate() {
        if *c != '@' || !calls(i, &chars) {
            continue;
        }
        let word: String = chars[i + 1..].iter().take_while(|c| in_name(**c)).collect();
        let Some(owed) = whom(word.trim_end_matches('-'), members) else {
            continue;
        };
        if !found.contains(&owed) {
            found.push(owed);
        }
    }
    found
}

/// Whether the `@` at `i` opens a name: a letter before it makes it an address.
fn calls(i: usize, chars: &[char]) -> bool {
    let Some(before) = i.checked_sub(1).and_then(|j| chars.get(j)) else {
        return true;
    };
    !(before.is_alphanumeric() || *before == '_')
}

/// Whether a character is still part of the name after an `@`.
fn in_name(c: char) -> bool {
    c.is_alphanumeric() || c == '_' || c == '-'
}

/// Who the word after an `@` names: one member — spelled as the roster spells
/// it — the room, or nobody. The roster answers first, so a member actually
/// named `all` keeps its own name and the sigil is only what is left over.
fn whom(word: &str, members: &[String]) -> Option<Owed> {
    match members.iter().find(|member| same(member, word)) {
        Some(member) => Some(Owed::Member(member.clone())),
        None => same(word, ALL).then_some(Owed::Room),
    }
}

fn is_member(who: &str, members: &[String]) -> bool {
    members.iter().any(|member| same(member, who))
}

/// The first line of a post, clipped: what a nudge quotes back.
pub fn head(text: &str) -> String {
    let line = text.lines().next().unwrap_or("").trim();
    if line.chars().count() <= HEAD {
        return line.to_string();
    }
    let kept: String = line.chars().take(HEAD - 1).collect();
    format!("{kept}…")
}

/// What a room owes, as one snapshot of it has it. The posts and the
/// membership come from the same place, so neither can drift from the other.
pub fn of_state(state: &SessionState) -> Vec<Mention> {
    let posts: Vec<Post> = state
        .transcript
        .items()
        .iter()
        .filter_map(|item| Post::of(item))
        .collect();
    mentions(&posts, &room::members_of(state))
}
