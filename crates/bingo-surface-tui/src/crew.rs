//! The active descendants of the session on screen, derived in tree order.
use crate::tree::{self, Scope, Tree};
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

#[cfg(test)]
mod tests;
