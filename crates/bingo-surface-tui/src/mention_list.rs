//! The `@` list over the input: the teammates a mention reaches, then the
//! files under the session's directory with what git says of them, ranked by
//! the one dock ranking ([`bingo_sdk::rank`]) and drawn by the shared list
//! builder with no count.

mod files;

pub use files::Files;

use bingo_collab::lifecycle::Lifecycle;
use bingo_sdk::{
    View,
    glyph::{Glyph, GlyphSet},
    pick, rank,
    sheet::{Sheet, Tally},
};

use crate::git_changes::Change;
use crate::mentions::Target;
use crate::tree::Tree;
use crate::ui::Ui;

/// The most rows the list shows at once.
const SHOWN: usize = 6;
/// The most matches kept after ranking.
const KEPT: usize = 40;
/// What the keybar says while the list is open.
pub const HINTS: [(&str, &str); 2] = [("↑↓", "choose"), ("tab", "insert")];

/// One thing a mention can name.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Entry {
    Agent {
        name: String,
        ink: &'static str,
        state: Option<Lifecycle>,
    },
    Everyone,
    File {
        path: String,
        change: Option<Change>,
    },
}

impl Entry {
    /// What follows the `@` once the entry is inserted.
    pub fn text(&self) -> &str {
        match self {
            Self::Agent { name, .. } => name,
            Self::Everyone => "all",
            Self::File { path, .. } => path,
        }
    }
}

/// An entry the query matched, with the characters it matched.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Offer {
    pub entry: Entry,
    pub hits: Vec<usize>,
}

/// The query being typed after an `@`, while the list is up: not after esc
/// closed it, and not while the palette or the switcher owns the dock.
pub fn query(ui: &Ui) -> Option<&str> {
    if ui.menu.dismissed || crate::palette::active(ui) || crate::sessions::active(ui) {
        return None;
    }
    crate::complete::mention(ui.composer.text())
}

/// What the list offers for the query, best first.
pub fn offered(ui: &Ui, tree: &Tree) -> Vec<Offer> {
    query(ui).map_or_else(Vec::new, |query| {
        ranked(
            query,
            &pool(crate::mentions::targets(tree), tree, &ui.files),
        )
    })
}

/// The offer the cursor rests on.
pub fn chosen(ui: &Ui, tree: &Tree) -> Option<Offer> {
    let offers = offered(ui, tree);
    let at = ui.menu.selected.min(offers.len().saturating_sub(1));
    offers.into_iter().nth(at)
}

/// The teammates first, in their own ink, then the files.
fn pool(targets: Vec<Target>, tree: &Tree, files: &Files) -> Vec<Entry> {
    let faces = crate::face::Faces::of(tree);
    targets
        .into_iter()
        .map(|target| match target {
            Target::Agent { name, state } => Entry::Agent {
                ink: faces.who(&name),
                name,
                state,
            },
            Target::Everyone => Entry::Everyone,
        })
        .chain(files.entries())
        .collect()
}

/// The entries the query matches, best first, at most [`KEPT`].
pub fn ranked(query: &str, pool: &[Entry]) -> Vec<Offer> {
    rank::ranked(query, pool, Entry::text)
        .into_iter()
        .take(KEPT)
        .map(|(entry, fuzzy)| Offer {
            entry: entry.clone(),
            hits: fuzzy.hits,
        })
        .collect()
}

/// The list as the shared builder draws it: six rows at most, no count, and
/// what it says when nothing matches.
pub fn draw(
    query: &str,
    offers: &[Offer],
    selected: usize,
    room: (u16, u16),
    glyphs: GlyphSet,
) -> View {
    let at = selected.min(offers.len().saturating_sub(1));
    let rows = offers
        .iter()
        .enumerate()
        .map(|(index, offer)| row(index, offer, index == at, glyphs))
        .collect::<Vec<_>>();
    let sheet = Sheet {
        head: String::new(),
        fact: None,
        top: offers
            .is_empty()
            .then(|| pick::text(format!("nothing matches @{query}"), "dim"))
            .into_iter()
            .collect(),
        selected: rows.get(at).map(|row| row.id.clone()),
        rows,
        foot: Vec::new(),
        pick: true,
    };
    sheet.list(room.0, room.1, SHOWN, Tally::Uncounted, glyphs)
}

fn row(index: usize, offer: &Offer, picked: bool, glyphs: GlyphSet) -> pick::Row {
    let (left, right) = match &offer.entry {
        Entry::Agent { name, ink, state } => (
            pick::marked(name, &offer.hits, ink, |_| ink),
            Some(pick::text(agent(*state, glyphs), "dim")),
        ),
        Entry::Everyone => (
            pick::marked("all", &offer.hits, "plan", |_| "plan"),
            Some(pick::text("everyone seated", "dim")),
        ),
        Entry::File { path, change } => (
            file(path, &offer.hits, picked),
            change.map(|change| {
                let (word, ink) = change.word();
                pick::text(word, ink)
            }),
        ),
    };
    let row = pick::Row::new(format!("@{index}"), left);
    match right {
        Some(right) => row.right(right),
        None => row,
    }
}

/// `agent · working`: what a teammate is, and what it is doing where known.
fn agent(state: Option<Lifecycle>, glyphs: GlyphSet) -> String {
    match state {
        Some(state) => format!("agent{}{}", Glyph::Sep.spell(glyphs), state.word()),
        None => "agent".into(),
    }
}

/// A path with its folders dim, so the file's own name stands out; a matched
/// character is lit wherever it falls.
fn file(path: &str, hits: &[usize], picked: bool) -> View {
    let name_at = path.rfind('/').map_or(0, |at| path[..=at].chars().count());
    let ink = if picked { "fg" } else { "muted" };
    pick::marked(
        path,
        hits,
        "glow",
        |at| if at < name_at { "dim" } else { ink },
    )
}

#[cfg(test)]
mod tests;
