//! The active descendants of the session on screen, derived in tree order.
use crate::{
    clock::Now,
    tree::{self, Scope, Tree},
};
use bingo_collab::lifecycle::Lifecycle;
use bingo_sdk::SessionState;

pub fn active(tree: &Tree) -> Vec<&SessionState> {
    let under = tree::covered(tree, Scope::Under(tree.view()));
    tree.sessions()
        .filter(|state| under.contains(&&state.summary.id))
        .filter(|state| {
            matches!(
                Lifecycle::of_state(state),
                Some(Lifecycle::Working | Lifecycle::Asking)
            )
        })
        .collect()
}

/// The strip uses the installed command; disabling the mod restores the palette.
pub fn open(ui: &mut crate::ui::Ui, tree: &Tree, now: Now) -> Vec<crate::effect::Effect> {
    if ui.catalogs.commands.iter().any(|entry| entry.id == "crew") {
        crate::input::command_line(ui, tree, now, "/crew".into())
    } else {
        crate::palette::open(ui, bingo_sdk::component::PaletteOpen::Sessions, now)
    }
}

#[cfg(test)]
mod tests;
