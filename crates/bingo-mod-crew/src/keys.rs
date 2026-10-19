//! What a press on the crew sheet does (`SHEETS.crew.key`, `.enter`): move,
//! walk into the picked row, or close and write `@name ` for a teammate.
use crate::state::Entry;
use bingo_sdk::{SessionId, sheet::Press};

#[derive(Debug, PartialEq, Eq)]
pub enum Operation {
    Close,
    /// Pick another row, by its identity.
    Move(Option<String>),
    /// Close the sheet and show this session.
    Walk(SessionId),
    /// Close the sheet and write this into the composer.
    Mention(String),
    None,
}

/// `selected` is the identity the drawing that took the press had picked; a
/// row that is gone falls back to the first, as a sheet's pick does.
pub fn operation(entries: &[Entry], selected: Option<&str>, press: &Press) -> Operation {
    let at = |id: Option<&str>| {
        id.and_then(|id| entries.iter().position(|entry| entry.id().as_str() == id))
            .unwrap_or(0)
    };
    let picked = entries.get(at(selected));
    match press {
        Press::Pick(id) => entries
            .iter()
            .find(|entry| entry.id().as_str() == id)
            .map_or(Operation::None, |entry| Operation::Walk(entry.id().clone())),
        Press::Key(key) => match key.as_str() {
            "esc" | "ctrl+c" => Operation::Close,
            "up" | "down" => {
                let at = at(selected);
                let to = if key == "down" {
                    (at + 1).min(entries.len().saturating_sub(1))
                } else {
                    at.saturating_sub(1)
                };
                Operation::Move(entries.get(to).map(|entry| entry.id().to_string()))
            }
            "enter" => picked.map_or(Operation::None, |entry| Operation::Walk(entry.id().clone())),
            "tab" => picked
                .and_then(Entry::mate)
                .map_or(Operation::None, |mate| {
                    Operation::Mention(format!("@{} ", mate.name))
                }),
            _ => Operation::None,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::{
        Need, ordered,
        tests::{mate, place},
    };

    fn sheet() -> Vec<Entry> {
        ordered(
            vec![mate("scout", Need::Working), mate("ada", Need::Idle)],
            vec![place("design")],
        )
    }

    fn key(key: &str) -> Press {
        Press::Key(key.into())
    }

    #[test]
    fn arrows_move_and_enter_or_a_click_walks_in() {
        let entries = sheet();
        assert_eq!(
            operation(&entries, Some("ses_scout"), &key("down")),
            Operation::Move(Some("ses_ada".into()))
        );
        assert_eq!(
            operation(&entries, Some("ses_design"), &key("enter")),
            Operation::Walk(SessionId::from_raw("ses_design"))
        );
        assert_eq!(
            operation(&entries, None, &Press::Pick("ses_ada".into())),
            Operation::Walk(SessionId::from_raw("ses_ada"))
        );
        assert_eq!(
            operation(&entries, None, &Press::Pick("ses_gone".into())),
            Operation::None
        );
        assert_eq!(operation(&entries, None, &key("esc")), Operation::Close);
    }

    #[test]
    fn tab_mentions_a_teammate_and_never_a_room() {
        let entries = sheet();
        assert_eq!(
            operation(&entries, Some("ses_ada"), &key("tab")),
            Operation::Mention("@ada ".into())
        );
        assert_eq!(
            operation(&entries, Some("ses_design"), &key("tab")),
            Operation::None
        );
        assert_eq!(operation(&[], None, &key("enter")), Operation::None);
    }
}
