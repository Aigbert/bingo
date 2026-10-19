//! A command result keeps its original view and scrolls in the common sheet.
use crate::{
    clock::Now,
    effect::Effect,
    layout::Layout,
    ui::{Open, Ui},
    views,
};
use bingo_sdk::sheet::{CONTENT_GUTTERS, Sheet};
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::text::Line;

const SHEET_ROWS: usize = 4;
const MIN_ROWS: usize = 3;
const MAX_ROWS: usize = 40;
const HEAD_ROWS: usize = 2;
const PAGE_ROWS: isize = 8;

fn window(view: &bingo_sdk::View, width: u16, height: u16) -> (usize, usize) {
    let width = width.saturating_sub(CONTENT_GUTTERS);
    let count = views::render(view, width.into()).len();
    let room = usize::from(height)
        .saturating_sub(SHEET_ROWS)
        .clamp(MIN_ROWS, MAX_ROWS);
    (count, room)
}

pub fn draw(
    ui: &Ui,
    frame: &mut Layout,
    width: u16,
    height: u16,
    now: Now,
) -> Option<Vec<Line<'static>>> {
    let Open::Result { head, view, at } = ui.focus.open() else {
        return None;
    };
    let sheet = Sheet {
        head: head.clone(),
        fact: None,
        top: vec![view.clone()],
        rows: vec![],
        selected: None,
        foot: vec![],
        pick: false,
    }
    .view(width, height, crate::theme::glyphs::set());
    let lines = frame.collect(views::marked(&sheet, width.into(), &views::Marks::at(now)));
    let (count, room) = window(view, width, height);
    let start = (*at).min(count.saturating_sub(room));
    let head = lines.iter().take(HEAD_ROWS).cloned().collect::<Vec<_>>();
    Some(
        head.into_iter()
            .chain(lines.into_iter().skip(HEAD_ROWS + start).take(room))
            .collect(),
    )
}

pub fn keys(ui: &mut Ui, key: KeyEvent, now: Now) -> Vec<Effect> {
    if key.code == KeyCode::Esc {
        ui.focus.close(now.instant);
        return vec![];
    }
    let area = ui.shown.area();
    let height = crate::layout::grid::dock_cap(usize::from(area.height));
    if let Some(Open::Result { view, at, .. }) = ui.focus.open_mut() {
        let (count, room) = window(view, area.width, height);
        let step = match key.code {
            KeyCode::Up => -1,
            KeyCode::Down => 1,
            KeyCode::PageUp => -PAGE_ROWS,
            KeyCode::PageDown => PAGE_ROWS,
            _ => return vec![],
        };
        *at = at
            .saturating_add_signed(step)
            .min(count.saturating_sub(room));
    }
    vec![]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::*;
    use crossterm::event::KeyModifiers;

    #[test]
    fn up_scrolls_immediately_after_down_reaches_the_last_window() {
        let (mut ui, now) = scene();
        ui.focus.show(
            Open::Result {
                head: "/result".into(),
                view: bingo_sdk::View::text(
                    (0..50)
                        .map(|i| format!("row {i}"))
                        .collect::<Vec<_>>()
                        .join("\n"),
                ),
                at: 0,
            },
            now.instant,
        );
        draw_sized(80, 24, &state(), &mut ui, now);
        for _ in 0..100 {
            super::keys(
                &mut ui,
                KeyEvent::new(KeyCode::Down, KeyModifiers::NONE),
                now,
            );
        }
        let Open::Result { at: before, .. } = ui.focus.open() else {
            panic!("result")
        };
        let before = *before;
        super::keys(&mut ui, KeyEvent::new(KeyCode::Up, KeyModifiers::NONE), now);
        let Open::Result { at, .. } = ui.focus.open() else {
            panic!("result")
        };
        assert_eq!(*at + 1, before);
    }
}
