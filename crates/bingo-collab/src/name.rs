//! What a room is called. One name has to serve as a title (`#design`) and as
//! a segment of the key `rooms/<parent>/design`, so whitespace and `/` are
//! refused here rather than mangled into something nobody asked for.

use bingo_sdk::{ErrorCode, KernelError};

/// The word a child uses for the session that spawned it. A post whose author
/// left no principal came from the person's own session, and that is what the
/// members of the room call it. On a roster it names the room's own holder
/// (ADR-0028 §1): no agent may take the name, so nothing else can mean it.
pub const PARENT: &str = "parent";

/// Two names, as a room compares them: the same word in any case. Every reader
/// of a name — the mention fold, the roster, the fan-out — comes through here.
pub fn same(a: &str, b: &str) -> bool {
    a.to_lowercase() == b.to_lowercase()
}

/// Whether a name on a roster means the session the room hangs under.
pub fn is_holder(member: &str) -> bool {
    same(member, PARENT)
}

/// The name a seat's posts sign with: its own title where it has one to be
/// addressed by, and `parent` for a session nobody spawned. Having no parent
/// is the fact; having no title used to stand in for it, and stopped being
/// able to when a session's first ask began naming it (M32).
pub fn signed_by(seat: &bingo_sdk::SessionSummary) -> String {
    match seat.parent.is_some() {
        true => seat.title.clone().unwrap_or_else(|| PARENT.to_string()),
        false => PARENT.to_string(),
    }
}

/// The word a person's door spends on ending a room (ADR-0053 §4). It is a
/// name no room may take, because `/room close design` has to mean the end of
/// `#design` rather than a room called `close` seating a member called
/// `design`.
pub const CLOSE: &str = "close";

/// A name a room can be opened under.
pub fn check_room(name: &str) -> Result<&str, KernelError> {
    let name = name.trim();
    let bad = name.is_empty() || name.contains('/') || name.chars().any(char::is_whitespace);
    if bad {
        return Err(KernelError::new(
            ErrorCode::InvalidInput,
            format!("{name:?} is not a room name: one word, no slashes"),
        ));
    }
    if same(name, CLOSE) {
        return Err(KernelError::new(
            ErrorCode::InvalidInput,
            format!(
                "{CLOSE:?} is the word that ends a room — `/room {CLOSE} <name>` — so no room \
                 is called it; pick another name"
            ),
        ));
    }
    Ok(name)
}

/// The name as a title. A room reads as a channel, and `#design` is how a
/// member is told where a message it received came from.
pub fn title(name: &str) -> String {
    format!("#{name}")
}


pub const ROOM: char = '#';

/// A validated agent name, retaining its display spelling.
#[derive(Clone, Debug)]
pub struct Name(String);

impl Name {
    pub fn parse(value: &str) -> Result<Self, KernelError> {
        agent_name(value).map(|value| Self(value.to_owned()))
    }
    pub fn as_str(&self) -> &str { &self.0 }
}
impl PartialEq for Name {
    fn eq(&self, other: &Self) -> bool { same(&self.0, &other.0) }
}
impl Eq for Name {}
impl std::fmt::Display for Name {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

fn agent_name(name: &str) -> Result<&str, KernelError> {
    let name = name.trim();
    let bad = name.is_empty() || name.contains('/') || name.chars().any(char::is_whitespace);
    if bad {
        return Err(KernelError::new(
            ErrorCode::InvalidInput,
            format!("{name:?} is not a name: one word, no slashes"),
        ));
    }
    if same(name, PARENT) {
        return Err(KernelError::new(
            ErrorCode::InvalidInput,
            format!("{PARENT:?} is what a child calls whoever spawned it; pick another name"),
        ));
    }
    if name.starts_with(ROOM) {
        return Err(KernelError::new(
            ErrorCode::InvalidInput,
            format!("{name:?} reads as a room; an agent's name starts with a letter"),
        ));
    }
    Ok(name)
}

