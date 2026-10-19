//! A teammate's ink follows the home tree's roster order, the rule
//! `bingo_collab::face` gives every owner; this only reads it off the tree.
use crate::{theme, tree::Tree};
use bingo_sdk::{SessionId, SessionSummary};
use ratatui::style::Style;

/// The home tree's sessions, as `bingo_collab::face` orders them, and the
/// names a mention can match, each in the ink its session wears.
#[derive(Clone, Debug, Default)]
pub struct Faces {
    home: String,
    rows: Vec<SessionSummary>,
    names: Vec<crate::markdown::mentions::Name>,
    /// The sessions running a turn this frame.
    working: Vec<SessionId>,
}

impl Faces {
    pub fn of(tree: &Tree) -> Self {
        let mut faces = Self {
            home: tree.root_id().as_str().to_owned(),
            rows: tree.sessions().map(|state| state.summary.clone()).collect(),
            names: Vec::new(),
            working: tree
                .sessions()
                .filter(|state| state.busy())
                .map(|state| state.summary.id.clone())
                .collect(),
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

    /// The accent of a session's own view (`viewInk`): a teammate's ink in
    /// its session, ember everywhere else.
    pub fn view_ink(&self, id: &SessionId) -> &'static str {
        bingo_collab::face::accent(id, &self.home, &self.rows)
    }

    /// The ink of whoever signs with `name`: a post's author, an @mention.
    pub fn who(&self, name: &str) -> &'static str {
        bingo_collab::face::who(name, &self.home, &self.rows)
    }

    /// What a person calls a session: `bingo` for home, a teammate or a room
    /// by its name.
    pub fn called(&self, id: &SessionId) -> String {
        if id.as_str() == self.home {
            return "bingo".into();
        }
        self.rows
            .iter()
            .find(|row| &row.id == id)
            .and_then(|row| row.title.clone())
            .unwrap_or_else(|| id.to_string())
    }

    /// Whether the teammate that signs with `name` is running a turn.
    pub fn works(&self, name: &str) -> bool {
        self.teammate(name)
            .is_some_and(|row| self.working.contains(&row.id))
    }

    /// The home's teammates, in the order they were created.
    pub fn teammates(&self) -> impl Iterator<Item = &SessionSummary> {
        bingo_collab::face::teammates(&self.home, &self.rows)
    }

    /// The home's teammate that signs with `name`.
    pub fn teammate(&self, name: &str) -> Option<&SessionSummary> {
        bingo_collab::face::teammate(name, &self.home, &self.rows)
    }
}

pub fn style(ink: &str) -> Style {
    theme::foreground(ink).unwrap_or_default()
}

/// How long one breath of an edge light takes (`breatheInk`).
const BREATH: std::time::Duration = std::time::Duration::from_millis(1800);

/// An edge light breathing between `ink` and the coal under it, and when it
/// next changes: ember's own coal under glow, the ink a shade darker under
/// any other.
pub fn breathing(ink: &str, now: crate::clock::Now) -> (Style, Option<std::time::Instant>) {
    let palette = theme::palette();
    let Some(color) = palette.named(ink) else {
        return (style(ink), None);
    };
    let coal = match ink {
        "glow" => palette.coal(),
        _ => theme::ink::shaded(color),
    };
    let weight = crate::clock::breathe(now, BREATH);
    let next = now.motion.then(|| now.instant + crate::clock::FRAME);
    (theme::literal(theme::ink::mix(color, coal, weight)), next)
}

/// The mark a teammate's row and post wear, in that teammate's ink.
pub fn mark() -> &'static str {
    theme::glyphs::get("lamp")
}

#[cfg(test)]
mod tests {
    use crate::test_support::*;

    #[test]
    fn a_teammate_session_wears_its_ink_where_home_wears_glow() {
        let mut tree = spawned_tree(busy_child("reviewer"));
        let faces = super::Faces::of(&tree);
        let ink = faces.view_ink(&child_id());
        assert_eq!(ink, faces.ink(&child_id()));
        assert_eq!(faces.view_ink(tree.root_id()), "glow");
        tree.show(&child_id());
        let (mut ui, now) = scene();
        let painted = crate::painted::painted(80, 24, &tree, &mut ui, still(now));
        let accent = super::style(ink).fg.unwrap();
        assert!(
            painted.inks().contains(&accent),
            "the spinner and side light"
        );
    }
}
