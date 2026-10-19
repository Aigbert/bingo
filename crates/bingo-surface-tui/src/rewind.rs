//! `esc esc` on an empty composer: the turns of this transcript, newest first,
//! and `⏎` to go back to one (design §3 — the rewind picker is a card).
//!
//! The turns are derived from `state.transcript.items()` every frame: an item carries the
//! turn it belongs to, so the boundaries are already in the transcript and
//! nothing here keeps a second list of them.
//!
//! The picker is offered only where the session has a `/rewind` to run, which
//! keeps it honest wherever it is not registered. Since M67 the checkpoints
//! plugin registers one (`docs/plans/M67-the-turn-that-can-be-undone.md`,
//! ADR-0045) and the chord is live; nothing here changed for it, which was
//! the point of asking the catalogue rather than assuming.

use bingo_sdk::{CommandSpec, TurnId, View, pick, sheet::Sheet};
use std::collections::BTreeMap;
pub const COMMAND: &str = "rewind";
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Rewind {
    pub request: bingo_sdk::IntentId,
    pub selected: Option<TurnId>,
    pub touched: BTreeMap<TurnId, Option<bingo_services::checkpoints::Touched>>,
}
impl Default for Rewind {
    fn default() -> Self {
        Self {
            request: bingo_sdk::IntentId::mint(),
            selected: None,
            touched: BTreeMap::new(),
        }
    }
}
pub use bingo_sdk::transcript::Turn;
/// Whether this session has a `/rewind` to run. Nothing is offered that could
/// not be done.
pub fn offered(commands: &[CommandSpec]) -> bool {
    commands
        .iter()
        .any(|spec| spec.name == COMMAND || spec.aliases.iter().any(|name| name == COMMAND))
}

/// The line a chosen row submits.
pub fn line(turn: &Turn) -> String {
    format!("/{COMMAND} {}", turn.id.as_str())
}

pub fn sheet(turns: &[Turn], card: &Rewind, width: u16, height: u16) -> View {
    let selected = card
        .selected
        .as_ref()
        .or_else(|| turns.last().map(|t| &t.id));
    let rows = turns
        .iter()
        .rev()
        .map(|turn| {
            let name = if selected == Some(&turn.id) {
                pick::text(turn.label(), "fg")
            } else {
                pick::text(turn.label(), "muted")
            };
            let mut row = pick::Row::new(turn.id.as_str(), name);
            if let Some(Some(touched)) = card.touched.get(&turn.id) {
                let count = touched.files.iter().filter(|file| file.changed).count();
                row.right = Some(pick::text(
                    if count == 0 {
                        "no files".into()
                    } else {
                        format!("{count} {} back", if count == 1 { "file" } else { "files" })
                    },
                    "dim",
                ));
            }
            row
        })
        .collect();
    Sheet {
        head: "Rewind".into(),
        fact: Some(pick::text("to just before a prompt", "dim")),
        top: vec![
            pick::text("the conversation and the files both go back", "dim"),
            pick::text("", "dim"),
        ],
        rows,
        selected: selected.map(ToString::to_string),
        foot: vec![],
        pick: true,
    }
    .with_limit(width, height, 8, crate::theme::glyphs::set())
}
#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::*;
    use bingo_sdk::transcript::turns;
    use bingo_sdk::{ArgSpec, ItemStatus, SessionState, TurnId};

    fn spec(name: &str) -> CommandSpec {
        CommandSpec {
            opens: false,
            keys: vec![],
            hidden: false,
            name: name.into(),
            aliases: Vec::new(),
            hint: String::new(),
            args: ArgSpec::None,
            instant: true,
            family: "session".into(),
        }
    }

    fn transcript() -> SessionState {
        let mut state = state();
        state.transcript = vec![
            in_turn(
                "itm_1",
                "trn_1",
                user("itm_1", "what is in this workspace?"),
            ),
            in_turn(
                "itm_2",
                "trn_1",
                assistant("itm_2", "One package.", ItemStatus::Completed),
            ),
            in_turn("itm_3", "trn_2", user("itm_3", "write me a note")),
        ]
        .into_iter()
        .collect();
        state
    }

    /// The same item, stamped with the turn it belongs to.
    fn in_turn(id: &str, turn: &str, mut item: bingo_sdk::Item) -> bingo_sdk::Item {
        item.id = bingo_sdk::ItemId::from_raw(id);
        item.turn = Some(TurnId::from_raw(turn));
        item
    }

    #[test]
    fn rewind_uses_the_shared_builder_and_keeps_the_selected_turn() {
        let turns = turns(&transcript().transcript);
        let card = Rewind {
            selected: Some(TurnId::from_raw("trn_1")),
            ..Default::default()
        };
        let view = sheet(&turns, &card, 100, 18);
        assert!(
            view.fold()
                .contains("the conversation and the files both go back")
        );
        assert!(
            serde_json::to_string(&view)
                .unwrap()
                .contains("\"ground\":\"pick\"")
        );
        assert!(!view.fold().contains("no files"));
    }

    /// Data-driven, both ways: a session whose catalogue has a `rewind` is
    /// offered the card, and one built without the plugin that registers it
    /// is silent.
    #[test]
    fn nothing_is_offered_that_could_not_be_done() {
        assert!(!offered(&[spec("compact"), spec("model")]));
        assert!(offered(&[spec("compact"), spec(COMMAND)]));
        let mut aliased = spec("undo");
        aliased.aliases = vec![COMMAND.to_string()];
        assert!(offered(&[aliased]));
    }

    #[test]
    fn a_chosen_row_runs_the_command_on_its_own_turn() {
        assert_eq!(line(&turns(&transcript().transcript)[1]), "/rewind trn_2");
    }
}
