//! Catalogue facts decoded for one projection, never cached beside the entries.
use std::collections::BTreeMap;

use bingo_sdk::{
    CatalogEntry, ToolLook,
    component::{Step, StepState, Tier},
};
use serde_json::Value;

#[derive(Clone, Debug, Default)]
pub struct Looks(BTreeMap<String, (Option<ToolLook>, Tier)>);

impl Looks {
    pub fn of(entries: &[CatalogEntry]) -> Self {
        Self(
            entries
                .iter()
                .map(|entry| {
                    (
                        entry.id.clone(),
                        (entry.look(), Tier::of(entry.traits().as_ref())),
                    )
                })
                .collect(),
        )
    }

    /// The input field a tool's look names its addressee in.
    pub fn who(&self, name: &str) -> Option<&str> {
        self.0.get(name)?.0.as_ref()?.who.as_deref()
    }

    pub fn in_ui(ui: &crate::ui::Ui) -> Self {
        Self::of(
            ui.catalogs
                .values
                .get("tools")
                .map(Vec::as_slice)
                .unwrap_or_default(),
        )
    }
}

/// One resolution for transcript steps, asks, child rows and pager titles.
pub fn resolve(name: &str, input: &Value, tools: &Looks, ascii: bool) -> Step {
    let declared = tools.0.get(name);
    let row = bingo_sdk::look::row(
        name,
        input,
        declared.and_then(|(look, _)| look.as_ref()),
        ascii,
    );
    let (target, reads) = row
        .target
        .map(|(text, reads)| (text, Some(reads)))
        .unwrap_or_default();
    Step {
        mark: row.mark,
        verb: row.verb,
        target,
        reads,
        tier: declared.map(|(_, tier)| *tier).unwrap_or(Tier::of(None)),
        state: StepState::Done,
        upshot: None,
        answer: None,
        elapsed_ms: None,
        why: None,
        landed_at: None,
        teammate: None,
    }
}

#[cfg(test)]
pub(crate) use tests::{cases, declared};

#[cfg(test)]
mod tests {
    use super::*;
    pub(crate) fn cases() -> Vec<(&'static str, Value, ToolLook, String, &'static str)> {
        vec![
            (
                "Bash",
                serde_json::json!({"command":"echo first\necho second", "description":"wrong"}),
                ToolLook::new('$', "run").target("command", bingo_sdk::Reads::Command),
                "echo first\necho second".into(),
                "echo first",
            ),
            (
                "WebFetch",
                serde_json::json!({"url":format!("https://example.test/{}", "a".repeat(160)), "prompt":"wrong"}),
                ToolLook::new('↓', "fetch")
                    .ascii('v')
                    .target("url", bingo_sdk::Reads::Path),
                format!("https://example.test/{}", "a".repeat(160)),
                "https://example.test/aa…aaaaaaaaaaaaaaaaaaaaaaaa",
            ),
        ]
    }

    pub(crate) fn declared(name: &str, look: ToolLook) -> Looks {
        Looks::of(&[CatalogEntry {
            id: name.into(),
            label: name.into(),
            meta: serde_json::json!({"look":look,"traits":bingo_sdk::ToolTraits::read_only()}),
        }])
    }

    #[test]
    fn declared_targets_survive_long_and_multiline_inputs() {
        for (name, input, look, target, _) in cases() {
            let tools = declared(name, look.clone());
            let step = resolve(name, &input, &tools, false);
            assert_eq!(step.target, target);
            assert_eq!(step.mark, look.mark);
            assert_eq!(step.verb, look.verb);
            assert_eq!(step.reads, look.target.map(|t| t.reads));
            assert_eq!(step.tier, Tier::Lookup);
        }
    }
}
