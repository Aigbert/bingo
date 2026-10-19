use crate::test_support::ember::{Band, Capture, assert_regions, story};
use crate::test_support::*;
use bingo_sdk::{
    slot::{Place, Slot},
    *,
};
use serde_json::json;

pub(super) fn fixture(capture: &Capture) -> (SessionState, crate::ui::Ui) {
    let (mut state, _) = story::scene("done");
    let mut summary = state.summary.clone();
    summary.model = Some("opus-5.5".into());
    summary.provider = Some("anthropic".into());
    summary.effort = Some(Effort::High);
    summary.title = Some("steer 吞掉排队消息".into());
    state.apply(&frame(state.seq.0 + 1, Event::SessionUpdated { summary }));
    let (mut ui, _) = scene();
    ui.catalogs
        .values
        .insert("tools".into(), story::catalogue());
    ui.last_typed = Some(capture.now().wall);
    (state, ui)
}
pub(super) fn show(state: &mut SessionState, ui: &mut crate::ui::Ui, owner: &str, view: View) {
    let slot = Slot::Pane {
        hints: None,
        id: "sheet".into(),
        title: String::new(),
        focus: true,
        close_on_escape: true,
        hold_toasts: false,
        asked: true,
        place: vec![Place::Dock],
        size: bingo_sdk::slot::PaneSize::from_rows(18),
        view: view.into(),
    };
    state.apply(&frame(
        state.seq.0 + 1,
        signalled(owner, "sheet", bingo_sdk::slot::with(json!({}), [slot])),
    ));
    ui.sync_panes(state);
}

#[test]
fn m198_keys_closed_full_frame() {
    let capture = Capture::load("keys-closed");
    capture.render(|| {
        let (state, mut ui) = fixture(&capture);
        let size = capture.size();
        ui.focus
            .show(crate::ui::Open::Keys(None), capture.now().instant);
        crate::input::on_key(
            &mut ui,
            &solo(&state),
            key(crossterm::event::KeyCode::Esc),
            capture.now(),
        );
        let screen = drawn(
            size.width,
            size.height,
            &solo(&state),
            &mut ui,
            capture.now(),
        );
        assert_regions(&capture, Band::Frame, screen.buffer(), &ui.shown.regions);
    });
}

fn band_lines(buffer: &ratatui::buffer::Buffer, area: ratatui::layout::Rect) -> Vec<String> {
    (area.y..area.bottom())
        .map(|y| {
            (area.x..area.right())
                .map(|x| buffer[(x, y)].symbol())
                .collect::<String>()
                .trim_end()
                .to_owned()
        })
        .collect()
}
