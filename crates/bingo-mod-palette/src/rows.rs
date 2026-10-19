//! What the list holds: the commands and skills a `/` ranks, the sessions a
//! `/resume` reopens, and the arguments of a command being typed.
use bingo_sdk::component::{PaletteHere, PaletteSession};
use bingo_sdk::{CatalogEntry, CommandSpec, Driver, rank};
use std::collections::BTreeMap;

/// The family prefix every skill command carries, `skill/<shelf>`: the one
/// word of contract between the skills plugin and this list.
pub const SKILL_FAMILY: &str = "skill/";

/// The family of the commands that act on the room on screen: listed in an
/// open room and nowhere else.
pub const ROOM_FAMILY: &str = "rooms/here";

/// The order a bare `/` lists the commands in; the rest follow in the
/// catalogue's order, then the skills.
const ORDER: &[&str] = &[
    "model",
    "think",
    "compact",
    "status",
    "resume",
    "rewind",
    "permission",
    "agents",
    "tasks",
    "room",
    "team",
    "seat",
    "unseat",
    "close",
    "schedule",
    "wake",
    "plugins",
    "mcp",
    "memory",
    "experience",
    "rename",
    "provider",
    "login",
    "logout",
    "clear",
    "help",
];

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Target {
    Command(String),
    /// A skill's command; its row's value is the shelf it was read from.
    Skill(String),
    Session(String),
    View(String),
    Edit(String),
    Run(String),
}
/// One row. `now` is the value set right: a command's key, a skill's shelf,
/// a session's age; `marks` are the label's characters the query matched.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Row {
    pub label: String,
    pub hint: String,
    pub alert: Option<String>,
    pub now: String,
    pub marks: Vec<usize>,
    pub target: Target,
}

/// The commands and skills `query` matches, best first; a tie keeps the
/// order a bare `/` lists them in.
pub fn commands(
    entries: &[CatalogEntry],
    bindings: &BTreeMap<String, String>,
    query: &str,
    here: PaletteHere,
) -> Vec<Row> {
    let listed = listed(entries, here);
    rank::ranked(query, &listed, |spec| &spec.name)
        .into_iter()
        .map(|(spec, fuzzy)| command(spec, fuzzy.hits, bindings))
        .chain(by_other_spelling(&listed, entries, bindings, query))
        .collect()
}

/// The specs a `/` lists, in the order it lists them.
fn listed(entries: &[CatalogEntry], here: PaletteHere) -> Vec<CommandSpec> {
    let mut specs = entries
        .iter()
        .filter(|entry| entry.meta["foldedInto"].as_str().is_none())
        .filter_map(|entry| serde_json::from_value::<CommandSpec>(entry.meta.clone()).ok())
        .filter(|spec| !spec.hidden && works_here(spec, here))
        .collect::<Vec<_>>();
    specs.sort_by_key(|spec| {
        (
            spec.family.starts_with(SKILL_FAMILY),
            ORDER
                .iter()
                .position(|name| *name == spec.name)
                .unwrap_or(ORDER.len()),
        )
    });
    specs
}

/// Whether a command does anything in the session on screen: a room takes no
/// skill, and only an open room takes the commands that act on it.
fn works_here(spec: &CommandSpec, here: PaletteHere) -> bool {
    let room = here.driver == Driver::Log;
    match spec.family.as_str() {
        ROOM_FAMILY => room && !here.closed,
        family => !(room && family.starts_with(SKILL_FAMILY)),
    }
}

/// The commands found only by an alias, or by a command folded into them:
/// after every name that matched, since the person typed some other word.
fn by_other_spelling(
    listed: &[CommandSpec],
    entries: &[CatalogEntry],
    bindings: &BTreeMap<String, String>,
    query: &str,
) -> Vec<Row> {
    listed
        .iter()
        .filter(|spec| rank::fuzzy(query, &spec.name).is_none())
        .filter(|spec| {
            spellings(spec, entries)
                .iter()
                .any(|name| rank::fuzzy(query, name).is_some())
        })
        .map(|spec| command(spec, Vec::new(), bindings))
        .collect()
}

/// A command's aliases, and the names and aliases of the commands folded
/// into it.
fn spellings(spec: &CommandSpec, entries: &[CatalogEntry]) -> Vec<String> {
    let folded = entries
        .iter()
        .filter(|entry| entry.meta["foldedInto"].as_str() == Some(spec.name.as_str()))
        .filter_map(|entry| serde_json::from_value::<CommandSpec>(entry.meta.clone()).ok())
        .filter(|folded| !folded.hidden)
        .flat_map(|folded| std::iter::once(folded.name).chain(folded.aliases));
    spec.aliases.iter().cloned().chain(folded).collect()
}

fn command(spec: &CommandSpec, marks: Vec<usize>, bindings: &BTreeMap<String, String>) -> Row {
    let shelf = spec.family.strip_prefix(SKILL_FAMILY);
    Row {
        label: spec.name.clone(),
        hint: spec.hint.clone(),
        alert: None,
        now: match shelf {
            Some(shelf) => shelf.into(),
            None => key(&spec.name, bindings).unwrap_or_default(),
        },
        marks,
        target: match shelf {
            Some(_) => Target::Skill(spec.name.clone()),
            None => Target::Command(spec.name.clone()),
        },
    }
}

/// The chord the client binds to the bare command, as it is bound.
fn key(name: &str, bindings: &BTreeMap<String, String>) -> Option<String> {
    let line = format!("/{name}");
    bindings
        .iter()
        .find(|(_, bound)| **bound == line)
        .map(|(chord, _)| chord.clone())
}

/// The characters of `text` that `query` matches, when it matches.
pub(crate) fn marks(text: &str, query: &str) -> Option<Vec<usize>> {
    rank::fuzzy(query, text).map(|found| found.hits)
}

/// `row` when `query` matches its label, its marks on the letters matched.
pub(crate) fn filter(mut row: Row, query: &str) -> Option<Row> {
    row.marks = marks(&row.label, query)?;
    Some(row)
}

pub(crate) fn session(summary: &PaletteSession, current: &str) -> Row {
    Row {
        label: summary.label.clone(),
        hint: String::new(),
        alert: summary.attention.clone(),
        now: if summary.id.as_str() == current {
            "this one".into()
        } else {
            summary.detail.clone()
        },
        marks: vec![],
        target: Target::Session(summary.id.to_string()),
    }
}
pub fn conversation(sessions: &[PaletteSession], query: &str, current: &str) -> Vec<Row> {
    sessions
        .iter()
        .map(|summary| {
            let mut row = session(summary, current);
            row.target = Target::View(summary.id.to_string());
            row.hint = summary.detail.clone();
            if summary.id.as_str() != current {
                row.now.clear();
            }
            row
        })
        .filter_map(|row| filter(row, query))
        .collect()
}

pub(crate) fn all_sessions() -> Row {
    Row {
        label: "All sessions…".into(),
        hint: String::new(),
        alert: None,
        now: String::new(),
        marks: vec![],
        target: Target::Run("/sessions".into()),
    }
}

#[cfg(test)]
mod tests;
