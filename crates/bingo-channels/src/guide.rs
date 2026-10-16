//! What this plugin says about itself: the page the model reads as the
//! `guide-channels` skill (ADR-0054 §1).
//!
//! Its test is next to the code it describes, and it reads the settings this
//! plugin claims out of the schema those settings generate: a field added to
//! an adapter, or a policy added to a rule, fails here until the page has a
//! sentence for it.

use std::sync::Arc;

use bingo_sdk::{Contribution, Page, Pages, Registrar};

pub static PAGES: Pages = Pages(&[Page {
    name: "channels",
    description: "A session in a chat thread: the adapters and their \
                  `channels` settings, who may speak to the bot, what a \
                  message becomes and what its attachments become, `SendFile`, \
                  and the `bingo channels` and `bingo gateway` runs.",
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
    use crate::loopback::Loopback;
    use crate::settings::schema;
    use serde_json::Value;

    fn page() -> &'static Page {
        let [page] = PAGES.0 else {
            panic!("one page");
        };
        page
    }

    /// A word the page must spell exactly, in backticks: a settings key read
    /// as prose is a settings key nobody can copy.
    fn quoted(word: &str) -> bool {
        page().body.contains(&format!("`{word}`"))
    }

    /// Every field of every settings type this plugin claims, and every word
    /// one of them may hold, out of the schema the claim carries. The schema
    /// is generated from the types, so this is the settings themselves asking
    /// the page whether it has heard of them.
    fn claimed() -> Vec<String> {
        let mut found = Vec::new();
        walk(schema().as_value(), &mut found);
        walk(schemars::schema_for!(crate::Access).as_value(), &mut found);
        found.sort();
        found.dedup();
        found
    }

    fn walk(value: &Value, found: &mut Vec<String>) {
        match value {
            Value::Object(map) => {
                if let Some(Value::Object(properties)) = map.get("properties") {
                    found.extend(properties.keys().cloned());
                }
                if let Some(Value::Array(words)) = map.get("enum") {
                    found.extend(words.iter().filter_map(Value::as_str).map(str::to_string));
                }
                map.values().for_each(|value| walk(value, found));
            }
            Value::Array(items) => items.iter().for_each(|value| walk(value, found)),
            _ => {}
        }
    }

    #[test]
    fn the_page_names_every_settings_key_and_word_this_plugin_claims() {
        let claimed = claimed();
        assert!(claimed.iter().any(|key| key == "coalesce"));
        assert!(claimed.iter().any(|key| key == "policy"));
        let unsaid: Vec<String> = claimed.into_iter().filter(|word| !quoted(word)).collect();
        assert!(unsaid.is_empty(), "the page never says {unsaid:?}");
    }

    #[test]
    fn the_page_names_the_top_level_key_and_the_tool_this_plugin_registers() {
        let claim = MANIFEST.config.expect("the plugin claims settings");
        for (key, _) in claim.keys {
            assert!(quoted(key), "the page never says `{key}`");
        }
        for provided in MANIFEST.provides {
            if let Some(name) = provided.strip_prefix("tool:") {
                assert!(quoted(name), "the page never says the {name} tool");
            }
        }
    }

    #[test]
    fn the_page_names_every_adapter_this_crate_has() {
        let id = Loopback::ID;
        assert!(quoted(id), "the page never says the {id} adapter");
    }

    /// The local platform stays off until the documented switch is set.
    #[test]
    fn the_page_names_the_local_platform_switch() {
        let variable = crate::loopback::ENABLE_ENV;
        assert!(quoted(variable), "the page never says {variable}");
    }

    #[test]
    fn the_page_says_how_a_chat_is_run_from_a_shell() {
        for line in ["bingo channels", "bingo gateway"] {
            assert!(page().body.contains(line), "the page never says {line}");
        }
    }

    /// The settings layers are TOML (ADR-0058 §1), so every example a person
    /// is invited to paste is written in the format the file is.
    #[test]
    fn the_settings_example_is_toml_and_no_json_is_left() {
        for fence in ["```json", "```jsonc"] {
            assert!(!page().body.contains(fence), "the page still shows {fence}");
        }
        assert!(
            page().body.contains("```toml"),
            "the example is still shown"
        );
    }
}
