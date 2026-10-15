use crate::Cursor;
use bingo_sdk::{ItemId, component::BrowseProps};
#[derive(Debug, PartialEq, Eq)]
pub(crate) enum Operation {
    Stay,
    Close,
    Preview,
    Run(String),
    Pager(Option<ItemId>),
    Copy(Option<ItemId>),
}
pub(crate) fn press(cursor: &mut Cursor, props: &BrowseProps, key: &str) -> Operation {
    let key = key
        .strip_prefix("shift+")
        .filter(|text| text.chars().count() == 1)
        .unwrap_or(key);
    if key == "ctrl+c" {
        return Operation::Close;
    }
    if cursor.sheet.is_some() {
        return sheet(cursor, props, key);
    }
    match key {
        "esc" => return Operation::Close,
        "[" | "]" if cursor.editing.is_none() => {
            walk(cursor, props, key);
        }
        "n" | "N" if cursor.editing.is_none() && !props.matches.is_empty() => {
            cursor.current = (cursor.current as isize + if key == "n" { 1 } else { -1 })
                .rem_euclid(props.matches.len() as isize) as usize;
            cursor.selected = props.matches[cursor.current].turn.clone();
        }
        "r" if cursor.editing.is_none() => return Operation::Preview,
        "enter" if cursor.editing.is_some() => {
            cursor.query = cursor.editing.take().unwrap_or_default();
            cursor.current = 0;
            cursor.selected = props
                .matches
                .first()
                .and_then(|m| m.turn.clone())
                .or(cursor.selected.clone());
        }
        "enter" => return Operation::Pager(item(props)),
        "y" if cursor.editing.is_none() => return Operation::Copy(item(props)),
        "backspace" => {
            cursor
                .editing
                .get_or_insert_with(|| cursor.query.clone())
                .pop();
        }
        "/" if cursor.editing.is_none() => {
            cursor.editing = Some(String::new());
        }
        text if text.chars().count() == 1 => {
            cursor
                .editing
                .get_or_insert_with(String::new)
                .push_str(text);
        }
        _ => {}
    }
    Operation::Stay
}
fn walk(cursor: &mut Cursor, props: &BrowseProps, key: &str) {
    let at = props
        .turns
        .iter()
        .position(|t| Some(&t.id) == cursor.selected.as_ref())
        .unwrap_or(0);
    let next = at
        .saturating_add_signed(if key == "[" { -1 } else { 1 })
        .min(props.turns.len().saturating_sub(1));
    cursor.selected = props.turns.get(next).map(|t| t.id.clone());
    cursor.query.clear();
    cursor.current = 0;
}
fn item(props: &BrowseProps) -> Option<ItemId> {
    props.matches.get(props.current).map(|m| m.item.clone())
}
fn sheet(cursor: &mut Cursor, props: &BrowseProps, key: &str) -> Operation {
    match key {
        "esc" => {
            cursor.sheet = None;
        }
        "up" | "pgup" => {
            cursor.sheet_top = cursor
                .sheet_top
                .saturating_sub(if key == "up" { 1 } else { 5 })
        }
        "down" | "pgdn" => {
            cursor.sheet_top = (cursor.sheet_top + if key == "down" { 1 } else { 5 }).min(
                cursor
                    .sheet
                    .as_ref()
                    .map_or(0, |s| s.files.len().saturating_sub(1)),
            )
        }
        "left" | "right" => cursor.mode = cursor.mode.step(if key == "left" { -1 } else { 1 }),
        "enter" => {
            if let Some(turn) = &props.selected {
                return Operation::Run(format!("/rewind {turn} {}", cursor.mode.name()));
            }
        }
        _ => {}
    }
    Operation::Stay
}

#[cfg(test)]
mod tests {
    use super::*;
    use bingo_sdk::{
        Action, SessionId, TurnId,
        transcript::{Match, Turn},
    };
    fn props() -> BrowseProps {
        BrowseProps {
            session: SessionId::from_raw("s"),
            selected: Some(TurnId::from_raw("t1")),
            query: String::new(),
            turns: ["t1", "t2"]
                .map(|id| Turn {
                    id: TurnId::from_raw(id),
                    asked: None,
                    first: 0,
                    last: 1,
                })
                .to_vec(),
            matches: vec![Match {
                item: ItemId::from_raw("i"),
                turn: Some(TurnId::from_raw("t2")),
                line: 2,
            }],
            current: 0,
            action: Action {
                name: "client".into(),
                args: serde_json::json!({}),
            },
            confirm: false,
        }
    }
    #[test]
    fn keys_walk_search_copy_page_and_back_out_of_the_sheet() {
        let props = props();
        let mut cursor = crate::handler::initial(&props);
        assert_eq!(press(&mut cursor, &props, "]"), Operation::Stay);
        assert_eq!(cursor.selected, Some(TurnId::from_raw("t2")));
        assert_eq!(
            press(&mut cursor, &props, "y"),
            Operation::Copy(Some(ItemId::from_raw("i")))
        );
        assert_eq!(
            press(&mut cursor, &props, "enter"),
            Operation::Pager(Some(ItemId::from_raw("i")))
        );
        for key in ["/", "n", "shift+R"] {
            press(&mut cursor, &props, key);
        }
        assert_eq!(cursor.editing.as_deref(), Some("nR"));
        press(&mut cursor, &props, "backspace");
        assert_eq!(cursor.editing.as_deref(), Some("n"));
        press(&mut cursor, &props, "enter");
        assert!(cursor.editing.is_none());
        assert_eq!(press(&mut cursor, &props, "r"), Operation::Preview);
        cursor.sheet = Some(Default::default());
        press(&mut cursor, &props, "right");
        assert_eq!(
            press(&mut cursor, &props, "enter"),
            Operation::Run("/rewind t1 conversation".into())
        );
        press(&mut cursor, &props, "esc");
        assert!(cursor.sheet.is_none());
        assert_eq!(press(&mut cursor, &props, "esc"), Operation::Close);
        assert_eq!(press(&mut cursor, &props, "ctrl+c"), Operation::Close);
    }
}
