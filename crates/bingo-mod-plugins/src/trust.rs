use bingo_sdk::{CatalogEntry, PluginState};
pub const TIERS: [(&str, &str); 3] = [
    ("tighten", "deny, ask, change your prompt"),
    ("rewrite", "also change tool inputs and results"),
    ("full", "also allow a call without asking"),
];
#[derive(Default)]
pub struct Choice {
    pub selected: usize,
    pub confirm: bool,
}
pub fn press(choice: &mut Choice, p: &CatalogEntry, key: &str) -> Option<crate::verbs::Verb> {
    let waiting = matches!(crate::groups::state(p), PluginState::Waiting { .. });
    match key {
        "up" => {
            choice.selected = choice.selected.saturating_sub(1);
            choice.confirm = false;
        }
        "down" => {
            choice.selected = (choice.selected + 1).min(if waiting { 3 } else { 2 });
            choice.confirm = false;
        }
        "esc" => return Some(crate::verbs::Verb::None),
        "enter" if choice.selected == 3 => {
            return Some(crate::verbs::Verb::Message(format!(
                "{} stays off",
                crate::groups::name(p)
            )));
        }
        "enter" if choice.selected == 2 && !choice.confirm => choice.confirm = true,
        "enter" => {
            return Some(crate::verbs::Verb::Line(format!(
                "/plugins {} {} {}",
                if waiting { "approve" } else { "trust" },
                p.id,
                TIERS[choice.selected].0
            )));
        }
        _ => {}
    }
    None
}
