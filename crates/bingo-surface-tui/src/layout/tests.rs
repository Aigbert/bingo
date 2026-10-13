use super::{Layout, layout};
use crate::test_support::*;
use ratatui::layout::Rect;

const SIZE: Rect = Rect::new(0, 0, 120, 40);

#[test]
fn layout_is_pure_and_idle_has_no_deadline() {
    let (ui, now) = scene();
    let tree = solo(&state());
    let a = layout(Layout::default(), &tree, &ui, SIZE, now);
    let b = layout(Layout::default(), &tree, &ui, SIZE, now);
    assert_eq!(a.canvas, b.canvas);
    assert_eq!(a.regions, b.regions);
    assert_eq!(a.cells, b.cells);
    assert_eq!(a.next_frame, None);
    assert_eq!(b.next_frame, None);
}

#[test]
fn layout_renders_each_band_once_and_reuses_unchanged_blocks() {
    let (ui, now) = scene();
    let mut state = state();
    state.transcript = (0..10_000)
        .map(|i| assistant(&format!("itm_{i}"), "A finished answer.", bingo_sdk::ItemStatus::Completed))
        .collect();
    let tree = solo(&state);
    let first = layout(Layout::default(), &tree, &ui, SIZE, now);
    assert_eq!(first.blocks.renders(), 10_000);
    assert_eq!(first.renders, [1; 5]);
    let next = layout(first, &tree, &ui, SIZE, now);
    assert_eq!(next.blocks.renders(), 10_000);
    assert_eq!(next.renders, [1; 5]);
    assert_eq!(next.canvas.buffer.content.len(), 120 * 40);
}

#[test]
fn layout_finds_a_rail_card_without_a_terminal() {
    let (mut ui, now) = scene();
    pin_board(&mut ui);
    let frame = layout(Layout::default(), &solo(&boarded()), &ui, SIZE, now);
    assert!(frame.regions.rail.is_some());
    assert!(frame.rail.iter().any(|(id, rows)| *id == demo_card("board") && !rows.is_empty()));
}

#[test]
fn layout_schedules_exit_at_its_deadline() {
    let (mut ui, now) = scene();
    ui.armed = Some(now.instant);
    let frame = layout(Layout::default(), &solo(&state()), &ui, SIZE, now);
    assert_eq!(frame.next_frame, Some(now.instant + crate::ui::EXIT_WINDOW));
}

#[test]
fn layout_schedules_a_scroll_sample() {
    let (mut ui, now) = scene();
    ui.scroll = crate::scroll::Scroll::Held { from: 30, to: 0, since: now.instant };
    let frame = layout(Layout::default(), &solo(&state()), &ui, SIZE, now);
    assert_eq!(frame.next_frame, Some(now.instant + crate::clock::FRAME));
}
