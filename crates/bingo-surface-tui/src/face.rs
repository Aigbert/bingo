//! A teammate's ink follows the home tree's roster order, the rule
//! `bingo_collab::face` gives every owner; this only reads it off the tree.
use crate::{clock, theme, tree::Tree};
use bingo_sdk::{SessionId, SessionSummary};
use ratatui::style::Style;

/// The home tree's sessions, as `bingo_collab::face` orders them, and the
/// names a mention can match, each in the ink its session wears.
#[derive(Clone, Debug, Default)]
pub struct Faces {
    home: String,
    rows: Vec<SessionSummary>,
    names: Vec<crate::markdown::mentions::Name>,
}

impl Faces {
    pub fn of(tree: &Tree) -> Self {
        let mut faces = Self {
            home: tree.root_id().as_str().to_owned(),
            rows: tree.sessions().map(|state| state.summary.clone()).collect(),
            names: Vec::new(),
        };
        faces.names = crate::markdown::mentions::known(tree, &faces);
        faces
    }

    /// Every name a mention can match: the tree's sessions and the holder.
    pub fn names(&self) -> &[crate::markdown::mentions::Name] {
        &self.names
    }

    /// The ink of one session of the tree.
    pub fn ink(&self, id: &SessionId) -> &'static str {
        bingo_collab::face::ink(id, &self.home, &self.rows)
    }

    /// The ink of whoever signs with `name`: a post's author, an @mention.
    pub fn who(&self, name: &str) -> &'static str {
        bingo_collab::face::who(name, &self.home, &self.rows)
    }
}

pub fn style(ink: &str) -> Style {
    theme::foreground(ink).unwrap_or_default()
}

/// Weight pulses without ever borrowing a state colour for an identity.
pub fn pulse(ink: &str, live: bool, now: clock::Now) -> Style {
    let style = style(ink);
    if live && now.motion && clock::alternating(now, theme::PULSE) {
        style.patch(theme::bold())
    } else {
        style
    }
}

/// The mark a teammate's row and post wear, in that teammate's ink.
pub fn mark() -> &'static str {
    theme::glyphs::get("lamp")
}
