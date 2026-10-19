//! The keys page projects the binding table; it owns no second chord list.
use bingo_sdk::{
    View,
    pick::{self, Row},
    sheet::Sheet,
};
use crossterm::event::KeyCode;

const TWO_COLUMNS: u16 = 90;
const COLUMN_COUNT: u16 = 2;
const COLUMN_GAP: u16 = 2;
const CHORD_COLUMNS: u16 = 11;
const PAGE_ROWS: usize = 8;
const MAX_ROWS: usize = 40;

pub fn scroll(selected: &mut Option<String>, width: u16, key: KeyCode) {
    let (down, count) = match key {
        KeyCode::Up => (false, 1),
        KeyCode::Down => (true, 1),
        KeyCode::PageUp => (false, PAGE_ROWS),
        KeyCode::PageDown => (true, PAGE_ROWS),
        _ => return,
    };
    let rows = lines(width);
    for _ in 0..count {
        *selected = pick::moved(&rows, selected.as_deref(), down);
    }
}

pub fn sheet(selected: Option<String>, columns: u16, rows: u16) -> View {
    let lines = lines(columns);
    Sheet {
        head: "Keys".into(),
        fact: Some(pick::text("/help lists every command", "dim")),
        top: vec![],
        rows: lines,
        selected,
        foot: vec![],
        pick: false,
    }
    .with_limit(columns, rows, MAX_ROWS, crate::theme::glyphs::set())
}
pub fn lines(columns: u16) -> Vec<Row> {
    let two = columns >= TWO_COLUMNS;
    let width = columns.saturating_sub(bingo_sdk::sheet::CONTENT_GUTTERS)
        / if two { COLUMN_COUNT } else { 1 };
    let groups = super::Group::ALL.map(|group| {
        group_rows(
            group,
            width.saturating_sub(if two { COLUMN_GAP } else { 0 }),
        )
    });
    let mut lines = vec![];
    for (i, pair) in groups
        .chunks(if two { usize::from(COLUMN_COUNT) } else { 1 })
        .enumerate()
    {
        if i > 0 {
            lines.push(Row::new(
                format!("gap:{}", pair[0][0].id),
                pick::text("", "dim"),
            ));
        }
        let count = pair.iter().map(Vec::len).max().unwrap_or(0);
        for j in 0..count {
            let children = pair
                .iter()
                .map(|rows| {
                    pick::cell(
                        rows.get(j)
                            .map(|row| row.left.clone())
                            .unwrap_or_else(|| pick::text("", "dim")),
                        width,
                    )
                })
                .collect();
            let Some(id) = pair
                .iter()
                .find_map(|rows| rows.get(j).map(|row| row.id.clone()))
            else {
                continue;
            };
            lines.push(Row::new(
                id,
                View::Box {
                    children,
                    layout: Box::new(bingo_sdk::view::BoxStyle {
                        flex_direction: Some(bingo_sdk::view::Direction::Row),
                        height: Some(1),
                        ..Default::default()
                    }),
                },
            ));
        }
    }
    lines
}

fn group_rows(group: super::Group, width: u16) -> Vec<Row> {
    let mut rows = vec![Row::new(group.words(), pick::text(group.words(), "dim"))];
    for binding in super::BINDINGS {
        let word = if binding.group == Some(group) {
            Some(binding.description)
        } else if group == super::Group::Working {
            binding.working
        } else {
            None
        };
        let Some(word) = word else {
            continue;
        };
        let keys = crate::theme::glyphs::chord(binding.keys);
        rows.push(Row::new(
            format!("{}:{}", group.words(), binding.keys),
            View::Box {
                children: vec![
                    pick::cell(pick::text(keys, "fg"), CHORD_COLUMNS),
                    pick::cell(
                        pick::text(word, "muted"),
                        width.saturating_sub(CHORD_COLUMNS),
                    ),
                ],
                layout: Box::new(bingo_sdk::view::BoxStyle {
                    flex_direction: Some(bingo_sdk::view::Direction::Row),
                    height: Some(1),
                    ..Default::default()
                }),
            },
        ));
    }
    rows
}

pub fn key(
    ui: &mut crate::ui::Ui,
    key: crossterm::event::KeyEvent,
    now: crate::clock::Now,
) -> Vec<crate::effect::Effect> {
    if key.code == KeyCode::Esc {
        ui.focus.close(now.instant);
    } else {
        let width = ui.shown.area().width;
        if let Some(crate::ui::Open::Keys(selected)) = ui.focus.open_mut() {
            crate::keys::sheet::scroll(selected, width, key.code);
        }
    }
    Vec::new()
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn keys_use_the_shared_window_with_binding_identities() {
        let rows = lines(80);
        let selected = "write:ctrl+v";
        assert!(rows.iter().any(|row| row.id == selected));
        assert!(lines(100).iter().any(|row| row.id == selected));
        let view = sheet(Some(selected.into()), 80, 8);
        let text = view.fold();
        assert!(text.contains("Keys"));
        assert!(text.contains("ctrl+v"));
        assert!(text.contains(&format!("4 of {}", rows.len())));
        assert!(!text.contains("▎"));
    }
    #[test]
    fn scrolling_keeps_binding_ids_when_width_changes() {
        let mut selected = None;
        scroll(&mut selected, 80, KeyCode::Down);
        assert_eq!(selected.as_deref(), Some("write:enter"));
        scroll(&mut selected, 100, KeyCode::Down);
        assert_eq!(selected.as_deref(), Some("write:shift+enter"));
        scroll(&mut selected, 80, KeyCode::PageDown);
        let rows = lines(80);
        assert_eq!(selected.as_deref(), Some(rows[10].id.as_str()));
    }
}
