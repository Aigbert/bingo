//! Where a skill came from, as a list shows it beside the name: the person's
//! own, the project's, the binary's, or a page a plugin wrote.
//!
//! It travels as the command's family, `skill/<word>`, so a client that groups
//! skills reads one prefix and shows the word after it as it is.

use std::path::Path;

use bingo_sdk::Env;

/// The family every skill command shares before its shelf.
pub const FAMILY: &str = "skill";

/// The shelf a skill was read from.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Shelf {
    /// `<config_dir>/skills`: the person's own, in every project.
    Yours,
    /// `.bingo/skills` somewhere from the working directory up.
    Project,
    /// In the binary.
    BuiltIn,
    /// A page a loaded plugin wrote about itself (ADR-0054 §2).
    Guide,
}

impl Shelf {
    /// A skill on disk is the person's when its layer is theirs, and the
    /// project's otherwise; one with no directory is in the binary.
    pub fn of(env: &Env, dir: &Path) -> Self {
        if dir.as_os_str().is_empty() {
            Self::BuiltIn
        } else if dir.parent() == Some(&env.config_dir.join(crate::layers::SKILLS)) {
            Self::Yours
        } else {
            Self::Project
        }
    }

    /// The word a list shows beside the skill.
    pub fn word(self) -> &'static str {
        match self {
            Self::Yours => "yours",
            Self::Project => "project",
            Self::BuiltIn => "built in",
            Self::Guide => "guide",
        }
    }

    /// The command family that carries this shelf.
    pub fn family(self) -> String {
        format!("{FAMILY}/{}", self.word())
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::*;

    #[test]
    fn a_skill_is_shelved_by_the_layer_it_was_read_from() {
        let env = Env::rooted("/home/user");
        for (dir, shelf) in [
            ("/home/user/.bingo/skills/deploy", Shelf::Yours),
            ("/work/repo/.bingo/skills/deploy", Shelf::Project),
            ("", Shelf::BuiltIn),
        ] {
            assert_eq!(Shelf::of(&env, &PathBuf::from(dir)), shelf, "{dir}");
        }
    }

    #[test]
    fn the_family_names_the_shelf_after_the_one_prefix() {
        assert_eq!(Shelf::Yours.family(), "skill/yours");
        assert_eq!(Shelf::BuiltIn.family(), "skill/built in");
        assert_eq!(Shelf::Guide.family(), "skill/guide");
    }
}
