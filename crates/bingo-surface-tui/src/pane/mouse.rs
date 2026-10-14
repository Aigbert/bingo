//! Pane hit testing precedes transcript selection and scrolling.
use super::{draw, moved, placed, seat::Seated};
use crate::{
    clock::Now,
    effect::Effect,
    focus::Focus,
    select::{Cell, Run},
    ui::Ui,
};
use bingo_sdk::{SessionState, slot::Place};
use crossterm::event::{MouseButton, MouseEvent, MouseEventKind};
use ratatui::layout::Position;
use unicode_width::UnicodeWidthStr;

pub fn mouse(
    ui: &mut Ui,
    state: &SessionState,
    event: MouseEvent,
    now: Now,
) -> Option<Vec<Effect>> {
    let at = Position::new(event.column, event.row);
    if event.kind == MouseEventKind::Down(MouseButton::Left)
        && let Some((key, _)) = ui.shown.waiting.iter().find(|(_, area)| area.contains(at))
    {
        let key = key.clone();
        ui.placed.panes.opened.insert(key.clone());
        if ui.shown.seating.collapsed.contains(&key) {
            ui.placed.panes.moved.insert(key.clone(), Place::Overlay);
        }
        ui.focus = Focus::Pane(key);
        return Some(vec![]);
    }
    let capture = matches!(
        event.kind,
        MouseEventKind::Drag(MouseButton::Left) | MouseEventKind::Up(MouseButton::Left)
    )
    .then(|| {
        ui.placed
            .panes
            .selection
            .as_ref()
            .map(|(key, _)| key.clone())
    })
    .flatten();
    let seat = capture
        .and_then(|key| ui.shown.seating.panes.iter().find(|seat| seat.key == key))
        .or_else(|| {
            ui.shown
                .seating
                .panes
                .iter()
                .rev()
                .find(|seat| seat.area.contains(at))
        })?
        .clone();
    Some(on_pane(ui, state, &seat, event, now))
}
fn on_pane(
    ui: &mut Ui,
    state: &SessionState,
    seat: &Seated,
    event: MouseEvent,
    now: Now,
) -> Vec<Effect> {
    match event.kind {
        MouseEventKind::Down(MouseButton::Left) => press(ui, state, seat, event, now),
        MouseEventKind::ScrollUp => scroll(ui, state, seat, false, now),
        MouseEventKind::ScrollDown => scroll(ui, state, seat, true, now),
        MouseEventKind::Drag(MouseButton::Left) => {
            let cell = cell(ui, state, seat, event, now);
            if let Some((_, run)) = &mut ui.placed.panes.selection {
                run.head = cell;
            }
        }
        MouseEventKind::Up(MouseButton::Left) => return copy(ui, state, seat, now),
        _ => {}
    }
    Vec::new()
}
fn press(ui: &mut Ui, state: &SessionState, seat: &Seated, event: MouseEvent, now: Now) {
    let focused = matches!(&ui.focus,Focus::Pane(key) if key==&seat.key);
    ui.select.dragging = None;
    ui.focus = Focus::Pane(seat.key.clone());
    ui.placed.panes.selection = None;
    if event.row == seat.area.y {
        let width = moved::label(seat.place).width() as u16;
        if focused && event.column >= seat.area.right().saturating_sub(width) {
            moved::cycle(ui, state, &seat.key);
        }
    } else {
        let anchor = cell(ui, state, seat, event, now);
        ui.placed.panes.selection = Some((
            seat.key.clone(),
            Run {
                anchor,
                head: anchor,
            },
        ));
    }
}
fn cell(ui: &Ui, state: &SessionState, seat: &Seated, event: MouseEvent, now: Now) -> Cell {
    let lines = placed(state)
        .iter()
        .find(|pane| pane.key == seat.key)
        .map_or(0, |pane| draw::lines(ui, state, pane, seat, now).len());
    let top = draw::offset(ui, seat, lines);
    Cell {
        line: top
            + usize::from(
                event
                    .row
                    .saturating_sub(seat.area.y + 1)
                    .min(seat.area.height.saturating_sub(2)),
            ),
        column: usize::from(
            event
                .column
                .saturating_sub(seat.area.x)
                .min(seat.area.width),
        ),
    }
}
fn scroll(ui: &mut Ui, state: &SessionState, seat: &Seated, down: bool, now: Now) {
    let Some(pane) = placed(state).into_iter().find(|pane| pane.key == seat.key) else {
        return;
    };
    let max = draw::lines(ui, state, &pane, seat, now)
        .len()
        .saturating_sub(usize::from(seat.area.height.saturating_sub(1)));
    let top = ui.placed.panes.scroll.entry(seat.key.clone()).or_default();
    *top = if down {
        top.saturating_add(crate::pointer::WHEEL as usize).min(max)
    } else {
        top.saturating_sub(crate::pointer::WHEEL as usize)
    };
}
fn copy(ui: &mut Ui, state: &SessionState, seat: &Seated, now: Now) -> Vec<Effect> {
    let Some((_, run)) = ui.placed.panes.selection.take() else {
        return vec![];
    };
    let Some(pane) = placed(state).into_iter().find(|pane| pane.key == seat.key) else {
        return vec![];
    };
    let lines = draw::lines(ui, state, &pane, seat, now)
        .iter()
        .map(ToString::to_string)
        .collect::<Vec<_>>();
    let text = run.text(&lines);
    if text.is_empty() {
        vec![]
    } else {
        vec![Effect::Copy(text)]
    }
}
