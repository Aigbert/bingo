//! What this plugin says about itself: the page the model reads as the
//! `guide-skills` skill (ADR-0054 §1).
//!
//! It is registered and gathered like any other plugin's — the skills plugin
//! reaches its own page through the catalogue and the service key, not through
//! a shortcut of its own. The map in `bundled/guide.md` is the other file: it
//! is the whole product's, and this one is this plugin's.

use std::sync::Arc;

use bingo_sdk::{Contribution, Page, Pages, Registrar};

pub static PAGES: Pages = Pages(&[Page {
    name: "skills",
    description: "What a skill is here: the layers a `SKILL.md` is read from, \
                  the frontmatter this product reads, what a body's \
                  placeholders stand for, and the two ways one is reached — \
                  the `Skill` tool and `/name`.",
    body: include_str!("guide.md"),
}]);

/// The page under the key every plugin registers its pages by.
pub fn contribution(registrar: &Registrar) -> Contribution {
    Contribution::service(&Pages::key(registrar.plugin_id()), Arc::new(PAGES), None)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::MANIFEST;

    fn page() -> &'static Page {
        let [page] = PAGES.0 else {
            panic!("one page");
        };
        page
    }

    #[test]
    fn the_page_names_every_tool_this_plugin_registers() {
        for provided in MANIFEST.provides {
            let Some(name) = provided.strip_prefix("tool:") else {
                continue;
            };
            assert!(
                page().body.contains(&format!("`{name}`")),
                "the page never says the {name} tool"
            );
        }
    }

    #[test]
    fn the_page_names_the_file_and_the_variable_a_skill_is_written_with() {
        for subject in [
            crate::scan::SKILL_FILE,
            "${BINGO_SKILL_DIR}",
            "$ARGUMENTS",
            "argument-hint",
            "guide-",
        ] {
            assert!(
                page().body.contains(subject),
                "the page never says {subject}"
            );
        }
    }
}
