//! Shift+tab: the permission mode cycle, the one line this surface types
//! on the person's behalf.

use super::*;

/// Shift+tab asks the policy for the next mode, as a typed `/permission`
/// would: the kernel decides, and the badge moves when the config frame lands.
/// Nothing is assumed here, so a refused command leaves the screen truthful.
pub(super) fn cycle_mode(ui: &mut Ui, state: &SessionState, now: Now) -> Vec<Effect> {
    let step = policy_step(state).or_else(|| placed_step(state));
    let Some(step) = step else {
        ui.notify(Level::Warn, UNKNOWN_MODE, now.instant);
        return Vec::new();
    };
    vec![Effect::CycleMode(step)]
}

/// The policy's own mode, from its description: the only cycle whose step
/// may be the person's typed line, and the one that wins over any other.
fn policy_step(state: &SessionState) -> Option<bingo_sdk::Input> {
    bingo_sdk::slot::described(state)
        .into_iter()
        .find_map(|placed| Some(mode_step(&placed.plugin, next_mode(placed.slot)?)))
}

/// A mode some other plugin placed is its own word: pressed as its placer,
/// so it can never stand for the person.
fn placed_step(state: &SessionState) -> Option<bingo_sdk::Input> {
    bingo_sdk::slot::placed(state)
        .into_iter()
        .find_map(|placed| {
            let action = next_mode(placed.slot)?;
            Some(crate::pressed(&placed.plugin, action))
        })
}

/// The item after `current`, wrapping.
fn next_mode(slot: bingo_sdk::slot::Slot) -> Option<bingo_sdk::Action> {
    let bingo_sdk::slot::Slot::Mode { current, modes } = slot else {
        return None;
    };
    let at = bingo_sdk::slot::mode_at(&current, &modes)?;
    modes
        .get((at + 1) % modes.len())
        .map(|item| item.action.clone())
}

/// The cycle is this surface's own key, and the one thing it types is a
/// `/permission` line naming a mode: that line is the person's. A mode the
/// policy made name anything else is the policy's word, not the person's.
pub(crate) fn mode_step(placer: &str, action: bingo_sdk::Action) -> bingo_sdk::Input {
    match action.args.as_str() {
        Some(mode) if action.name == PERMISSION && is_mode_word(mode) => {
            bingo_sdk::Input::text(format!("/{PERMISSION} {mode}"), crate::origin())
        }
        _ => crate::pressed(placer, action),
    }
}

/// The command shift+tab types.
const PERMISSION: &str = "permission";

fn is_mode_word(mode: &str) -> bool {
    !mode.is_empty() && mode.chars().all(|c| c.is_ascii_alphanumeric())
}
