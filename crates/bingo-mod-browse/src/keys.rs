use crate::Cursor;
use bingo_sdk::{ItemId, component::BrowseProps};
#[derive(Debug, PartialEq, Eq)]
pub(crate) enum Operation {
    Stay,
    Close,
    Preview,
    Notice(&'static str),
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
    if key == "preview" {
        return preview(props);
    }
    if cursor.sheet.is_some() {
        return sheet(cursor, props, key);
    }
    if key == "esc" {
        if cursor.editing.take().is_some() || !cursor.query.is_empty() {
            cursor.query.clear();
            cursor.current = 0;
            return Operation::Stay;
        }
        return Operation::Close;
    }
    if cursor.editing.is_some() {
        return edit(cursor, key);
    }
    match key {
        "[" | "]" => walk(cursor, props, key),
        "n" | "N" if !cursor.query.is_empty() => {
            if props.count > 0 {
                cursor.current = (cursor.current as isize + if key == "n" { 1 } else { -1 })
                    .rem_euclid(props.count as isize) as usize;
            }
        }
        "r" => return preview(props),
        "enter" => return Operation::Pager(item(props)),
        "y" => return Operation::Copy(item(props)),
        "/" => cursor.editing = Some(String::new()),
        "backspace" => {
            cursor.editing = Some(cursor.query.clone());
            return edit(cursor, key);
        }
        text if text.chars().count() == 1 => cursor.editing = Some(text.into()),
        _ => {}
    }
    Operation::Stay
}
fn edit(cursor: &mut Cursor, key: &str) -> Operation {
    match key {
        "enter" => {
            cursor.query = cursor.editing.take().unwrap_or_default();
            cursor.current = 0;
        }
        "backspace" => {
            if let Some(text) = &mut cursor.editing {
                text.pop();
            }
        }
        text if text.chars().count() == 1 => {
            if let Some(query) = &mut cursor.editing {
                query.push_str(text);
            }
        }
        _ => {}
    }
    Operation::Stay
}
fn preview(props: &BrowseProps) -> Operation {
    if props.busy {
        Operation::Notice("Wait for the running turn before rewinding")
    } else {
        Operation::Preview
    }
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
    props.hit.as_ref().map(|m| m.item.clone())
}
fn sheet(cursor: &mut Cursor, props: &BrowseProps, key: &str) -> Operation {
    if key == "esc" {
        cursor.sheet = None;
        return Operation::Stay;
    }
    let Some(shown) = cursor.sheet.as_ref() else {
        return Operation::Stay;
    };
    if shown.length != props.length {
        cursor.selected = shown.turns.first().map(|t| t.id.clone());
        cursor.sheet = None;
        return preview(props);
    }
    match key {
        "up" | "pgup" => {
            cursor.sheet_top = cursor
                .sheet_top
                .saturating_sub(if key == "up" { 1 } else { 5 })
        }
        "down" | "pgdn" => {
            cursor.sheet_top = (cursor.sheet_top + if key == "down" { 1 } else { 5 })
                .min(shown.touched.files.len().saturating_sub(1))
        }
        "left" | "right" => {
            cursor.mode = cursor.mode.step(if key == "left" { -1 } else { 1 });
            cursor.sheet_top = 0;
        }
        "enter" => {
            if props.busy {
                return preview(props);
            }
            if let Some(sheet) = crate::sheet::sheet(&shown.turns, &shown.touched, cursor.mode) {
                return Operation::Run(sheet.line);
            }
        }
        _ => {}
    }
    Operation::Stay
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use bingo_sdk::{
        Action, SessionId, TurnId,
        transcript::{Match, Turn},
    };
    pub(crate) fn props() -> BrowseProps {
        BrowseProps {
            session: SessionId::from_raw("s"),
            selected: Some(TurnId::from_raw("t1")),
            query: String::new(),
            turns: ["t1", "t2"]
                .map(|id| Turn {
                    id: TurnId::from_raw(id),
                    headline: None,
                    first: 0,
                    last: 1,
                })
                .to_vec(),
            hit: Some(Match {
                item: ItemId::from_raw("i"),
                turn: Some(TurnId::from_raw("t2")),
                line: 2,
            }),
            count: 1,
            length: 2,
            busy: false,
            previewed: false,
            refresh: false,
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
        cursor.sheet = Some(crate::Preview {
            turns: props.turns.clone(),
            length: props.length,
            touched: Default::default(),
        });
        press(&mut cursor, &props, "right");
        assert_eq!(
            press(&mut cursor, &props, "enter"),
            Operation::Run("/rewind t1 conversation".into())
        );
        press(&mut cursor, &props, "esc");
        assert!(cursor.sheet.is_none());
        assert_eq!(press(&mut cursor, &props, "esc"), Operation::Stay);
        assert_eq!(press(&mut cursor, &props, "esc"), Operation::Close);
        assert_eq!(press(&mut cursor, &props, "ctrl+c"), Operation::Close);
    }
    #[test]
    fn sheet_pins_its_turn_refreshes_on_growth_and_refuses_busy() {
        let mut props = props();
        let mut cursor = crate::handler::initial(&props);
        cursor.sheet = Some(crate::Preview {
            turns: props.turns.clone(),
            length: props.length,
            touched: Default::default(),
        });
        props.selected = Some(TurnId::from_raw("t2"));
        assert_eq!(
            press(&mut cursor, &props, "enter"),
            Operation::Run("/rewind t1 both".into())
        );
        props.busy = true;
        assert!(matches!(
            press(&mut cursor, &props, "enter"),
            Operation::Notice(_)
        ));
        props.busy = false;
        props.length += 1;
        assert_eq!(press(&mut cursor, &props, "enter"), Operation::Preview);
        assert!(cursor.sheet.is_none());
        assert_eq!(cursor.selected, Some(TurnId::from_raw("t1")));
    }
    #[test]
    fn typing_n_starts_search_enter_resets_match_and_escape_clears_first() {
        let props = props();
        let mut cursor = crate::handler::initial(&props);
        press(&mut cursor, &props, "n");
        assert_eq!(cursor.editing.as_deref(), Some("n"));
        cursor.current = 7;
        press(&mut cursor, &props, "enter");
        assert_eq!(cursor.query, "n");
        assert_eq!(cursor.current, 0);
        assert_eq!(press(&mut cursor, &props, "esc"), Operation::Stay);
        assert!(cursor.query.is_empty());
        assert_eq!(press(&mut cursor, &props, "esc"), Operation::Close);
    }
}
