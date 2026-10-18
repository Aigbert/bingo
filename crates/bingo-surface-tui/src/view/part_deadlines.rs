use super::*;
use crate::test_support::*;
use bingo_sdk::{Event, View};
use serde_json::json;

fn draw(width: u16, height: u16, tree: &Tree, ui: &mut Ui, now: Now) {
    theme::with(crate::painted::daylight(), || {
        crate::test_support::draw_tree(width, height, tree, ui, now);
    });
}

fn configured(events: Vec<Event>) -> SessionState {
    let mut config = state().config;
    config.kernel["render"] = json!({"Welcome":[{}],"Question":[{}]});
    folded(
        std::iter::once(Event::ConfigChanged { config })
            .chain(events)
            .enumerate()
            .map(|(i, event)| frame(i as u64 + 1, event))
            .collect(),
    )
}

fn entrance(ui: &mut Ui, key: crate::pulled::Key, now: Now) {
    let view: View = serde_json::from_value(json!({
        "kind":"Text", "color":"glow", "children":["arrival"],
        "motion":{"kind":"fade", "from":"bg", "atMs":now.wall.as_millisecond()+10,"ms":100}
    }))
    .unwrap();
    theme::with(crate::painted::daylight(), || {
        let drawing = views::marked(&view, 80, &views::Marks::at(now));
        assert_eq!(drawing.next, Some(later(now, 10).instant));
        assert!(
            views::marked(&view, 80, &views::Marks::at(still(now)))
                .next
                .is_none()
        );
    });
    ui.placed.pulled.request(&key);
    ui.placed.pulled.answer(key, view);
}

fn welcome_entrance(state: &SessionState, ui: &mut Ui, now: Now) {
    let measurements = Default::default();
    let rows =
        crate::transcript::Rows::of(state, 80, &ui.folds, &[], &measurements, &ui.linked, now);
    let key = crate::component::welcome::request(&rows).unwrap();
    entrance(ui, key, now);
}

#[test]
fn a_zero_row_transcript_keeps_the_inline_asks_entrance() {
    let state = configured(vec![opened(question(false, true))]);
    let (mut ui, born) = scene();
    let now = later(born, 10_000);
    let open = &state.interactions[0];
    let props = crate::component::question::props(open).unwrap();
    let key = crate::component::key(&state, open.id.as_str(), &props, 80, 0).unwrap();
    entrance(&mut ui, key, now);
    draw(80, 4, &solo(&state), &mut ui, now);
    assert_eq!(ui.shown.regions.transcript.height, 0);
    assert_eq!(ui.shown.deadline(), Some(later(now, 10).instant));
}

#[test]
fn a_zero_row_transcript_keeps_the_welcomes_entrance() {
    let state = configured(vec![]);
    let (mut ui, born) = scene();
    let now = later(born, 10_000);
    welcome_entrance(&state, &mut ui, now);
    draw(80, 4, &solo(&state), &mut ui, now);
    assert_eq!(ui.shown.regions.transcript.height, 0);
    assert_eq!(ui.shown.deadline(), Some(later(now, 10).instant));
}

#[test]
fn end_and_page_keys_schedule_the_scroll_flash_without_overflow() {
    use crossterm::event::KeyCode;
    let tree = solo(&state());
    for code in [KeyCode::End, KeyCode::PageDown] {
        let (mut ui, now) = scene();
        draw(80, 24, &tree, &mut ui, still(now));
        crate::input::on_key(&mut ui, &tree, key(code), still(now));
        assert_eq!(
            ui.scroll.next_change(still(now)),
            Some(later(now, 900).instant)
        );
        assert_eq!(
            ui.scroll.next_change(still(later(now, 899))),
            Some(later(now, 900).instant)
        );
        assert_eq!(ui.scroll.next_change(still(later(now, 900))), None);
    }
    let (_, now) = scene();
    let mut scroll = crate::scroll::Scroll::default();
    scroll.by(20, 100, 20, now.instant);
    assert_eq!(
        scroll.next_change(now),
        Some(now.instant + crate::clock::FRAME)
    );
    assert_eq!(
        scroll.next_change(later(now, 100)),
        Some(later(now, 900).instant)
    );
    assert_eq!(scroll.next_change(later(now, 900)), None);
}

#[test]
fn part_deadlines_do_not_depend_on_terminal_height() {
    let (mut ui, born) = scene();
    let now = later(born, 10_000);
    let mut state = configured(vec![started("turn")]);
    let mut ended = frame(3, completed("turn", bingo_sdk::TurnStatus::Completed));
    ended.ts = later(now, -100).wall;
    state.apply(&ended);
    welcome_entrance(&state, &mut ui, now);
    ui.notify(
        bingo_sdk::Level::Info,
        "saved",
        now.instant - std::time::Duration::from_millis(200),
    );
    ui.scroll
        .end(now.instant - std::time::Duration::from_millis(100));
    let tree = solo(&state);
    for (age, wake) in [(0, 10), (790, 800), (1100, 2200)] {
        for height in 1..=60 {
            // Resizing adds its own growth wake; each geometry starts settled here.
            ui.shown = Layout::default();
            draw(80, height, &tree, &mut ui, later(now, age));
            assert_eq!(
                ui.shown.deadline(),
                Some(later(now, wake).instant),
                "height {height}, age {age}"
            );
        }
    }
}
