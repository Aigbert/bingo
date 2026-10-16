//! The mod author's page, gathered as the `guide-mods` skill (ADR-0054).

use std::sync::Arc;

use bingo_sdk::{Contribution, Page, Pages, Registrar};

pub static PAGES: Pages = Pages(&[
    Page {
        name: "mods",
        description: "Write, fix or explain a mod: module discovery, hooks, events, the $ API, drawing and state, generated types, trust, testing and reload; plus external-process plugins.",
        body: include_str!("guide.md"),
    },
    Page {
        name: "mods-switches",
        description: "List, enable and disable plugins; immediate switches, next-start reasons, and repository approval.",
        body: include_str!("switches.md"),
    },
    Page {
        name: "mods-keys",
        description: "Bind a key to a command, own a focused pane's keys with onKey, or wrap a built-in feature's drawing from a mod.",
        body: include_str!("keys.md"),
    },
]);

/// The page under the key every plugin registers its pages by.
pub fn contribution(registrar: &Registrar) -> Contribution {
    Contribution::service(&Pages::key(registrar.plugin_id()), Arc::new(PAGES), None)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn page() -> &'static Page {
        let [page, ..] = PAGES.0 else {
            panic!("the mods page, then the keys page");
        };
        page
    }

    #[test]
    fn the_page_names_every_event_in_the_schema() {
        let schema: serde_json::Value =
            serde_json::from_str(include_str!("../../../schema/events.json")).unwrap();
        for event in schema["events"].as_object().unwrap().keys() {
            assert!(
                page().body.contains(&format!("`{event}`")),
                "the page never says {event}"
            );
        }
    }

    #[test]
    fn the_page_names_every_dollar_noun_in_the_schema_and_ui() {
        let schema: serde_json::Value =
            serde_json::from_str(include_str!("../../../schema/dollar.json")).unwrap();
        for method in schema["methods"].as_object().unwrap().keys() {
            let (noun, _) = method.split_once('.').unwrap();
            assert!(
                page().body.contains(&format!("`$.{noun}`")),
                "the page never says $.{noun}"
            );
        }
        assert!(page().body.contains("`$.ui`"));
    }

    #[test]
    fn the_page_names_the_checks_and_trust_tiers() {
        for subject in [
            "bingo plugins validate",
            "bingo plugins test",
            "'bingo/testing'",
            "`tighten`",
            "`rewrite`",
            "`full`",
        ] {
            assert!(
                page().body.contains(subject),
                "the page never says {subject}"
            );
        }
    }

    #[test]
    fn the_keys_page_names_the_setting_the_handler_and_the_placement() {
        let [_, _, keys] = PAGES.0 else {
            panic!("the mods page, then the keys page");
        };
        for subject in ["`keys`", "`onKey`", "Placement::Builtin", "bingo-mod-hello"] {
            assert!(
                keys.body.contains(subject),
                "the keys page never says {subject}"
            );
        }
        assert!(!keys.description.contains('\n'));
    }
}
