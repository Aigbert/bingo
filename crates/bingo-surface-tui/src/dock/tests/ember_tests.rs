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

#[test]
fn m198_permission_mode_band_uses_policy_meanings_and_session_rules() {
    let capture = Capture::load("model");
    capture.render(|| {
        let (mut state, mut ui) = fixture(&capture);
        let modes = [
            ("default", "default", "trusted read-only tools run; everything else asks"),
            ("acceptEdits", "accept edits", "edits inside the working directories run without a prompt"),
            ("plan", "plan", "nothing that is not read-only runs at all"),
            ("bypassPermissions", "bypass", "everything runs except what only a person may decide"),
        ].into_iter().map(|(id,label,meaning)| ActionItem { label:label.into(), meaning:Some(meaning.into()), key:None, action:Action { name:"permission".into(), args:json!(id) } }).collect();
        let mut config = state.config.clone();
        config.plugins.insert("bingo.permissions".into(), slot::with(json!({"mode":"default","rules":["Bash(cargo test:*)"]}), [Slot::Mode { current:"default".into(), modes }]));
        config.kernel[ConfigView::POLICY] = json!("bingo.permissions");
        state.apply(&frame(state.seq.0+1, Event::ConfigChanged { config }));
        ui.focus.show(crate::ui::Open::Mode(Some("plan".into())), capture.now().instant);
        let screen = drawn(100,32,&solo(&state),&mut ui,capture.now());
        let band = ui.shown.regions.composer;
        let lines = band_lines(screen.buffer(), band);
        assert!(lines.iter().any(|line| line.starts_with("   Permission mode") && line.ends_with("⇧tab cycles")), "{lines:?}");
        assert!(lines.iter().any(|line| line.trim() == "▎ plan           nothing that is not read-only runs at all"), "{lines:?}");
        assert!(lines.iter().any(|line| line.trim() == "Bash(cargo test:*)"));
        let y = band.y + lines.iter().position(|line| line.contains("▎ plan")).unwrap() as u16;
        assert_eq!(screen.buffer()[(3,y)].fg, crate::theme::palette().named("plan").unwrap());
        assert_eq!(screen.buffer()[(1,y)].bg, crate::theme::palette().named("pick").unwrap());
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

#[test]
fn m198_rewind_full_frame() {
    let capture = Capture::load("rewind");
    capture.render(|| {
        let (state, mut ui) = fixture(&capture);
        let touched = bingo_services::checkpoints::Touched {
            files: [
                "crates/bingo-core/src/session/queue.rs",
                "crates/bingo-core/tests/queue_steer.rs",
            ]
            .into_iter()
            .map(|path| bingo_services::checkpoints::File {
                path: path.into(),
                changed: true,
                fate: bingo_services::checkpoints::Fate::Restored,
            })
            .collect(),
            shell_calls: 2,
        };
        let card = crate::rewind::Rewind {
            selected: None,
            touched: bingo_sdk::transcript::turns(&state.transcript)
                .iter()
                .map(|turn| (turn.id.clone(), Some(touched.clone())))
                .collect(),
            ..Default::default()
        };
        ui.focus
            .show(crate::ui::Open::Rewind(card), capture.now().instant);
        let screen = drawn(100, 32, &solo(&state), &mut ui, capture.now());
        assert_regions(&capture, Band::Frame, screen.buffer(), &ui.shown.regions);
    });
}
