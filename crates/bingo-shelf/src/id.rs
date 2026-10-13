//! An entry's id: a short slug minted at creation, which is also its file
//! name, and the prefix every tool that takes an id accepts. What the index
//! shows is what a tool accepts (ADR-0014 §4); the old project's display-only
//! short id was accepted by no tool, so what a person could read they could
//! not name.

/// Long enough that a project's entries do not collide, short enough to type.
const LEN: usize = 8;

/// A fresh slug. The tail of a ULID is its random component — the head is the
/// clock, and a project's entries should not sort by the hour they were
/// written.
pub fn mint() -> String {
    let raw = ulid::Ulid::generate().to_string().to_lowercase();
    raw.chars()
        .skip(raw.chars().count().saturating_sub(LEN))
        .collect()
}

/// What a prefix named.
#[derive(Debug, PartialEq)]
pub enum Named<'a, T> {
    One(&'a T),
    Unknown,
    /// The ids it could have meant.
    Ambiguous(Vec<&'a str>),
}

/// The entry `prefix` names. An exact id always wins over the entries it is a
/// prefix of, so a hand-shortened file name can still be named in full.
pub fn resolve<'a, T>(
    entries: &'a [T],
    prefix: &str,
    key: impl Fn(&'a T) -> &'a str,
) -> Named<'a, T> {
    let prefix = prefix.trim();
    if prefix.is_empty() {
        return Named::Unknown;
    }
    if let Some(exact) = entries.iter().find(|entry| key(entry) == prefix) {
        return Named::One(exact);
    }
    let matched: Vec<&T> = entries
        .iter()
        .filter(|entry| key(entry).starts_with(prefix))
        .collect();
    match matched.as_slice() {
        [] => Named::Unknown,
        [one] => Named::One(one),
        many => Named::Ambiguous(many.iter().map(|entry| key(entry)).collect()),
    }
}
