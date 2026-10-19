use crate::test_support::ember::{Capture, story};
use crate::test_support::*;
use bingo_sdk::{
    slot::{Place, Slot},
    *,
};
use serde_json::json;

fn fixture(capture: &Capture) -> (SessionState, crate::ui::Ui) {
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
fn show(state: &mut SessionState, ui: &mut crate::ui::Ui, owner: &str, view: View) {
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
fn m198_wake_band_has_six_delays_and_only_a_pending_cancel() {
    let capture = Capture::load("model");
    capture.render(|| {
        for pending in [false, true] {
            let (mut state, mut ui) = fixture(&capture);
            let wake = pending.then(|| {
                bingo_schedule::wake::set(
                    capture.now().wall,
                    jiff::SignedDuration::from_mins(5),
                    "continue".into(),
                )
            });
            show(
                &mut state,
                &mut ui,
                "bingo.schedule",
                bingo_schedule::wake_sheet::tree(
                    wake.as_ref(),
                    None,
                    100,
                    19,
                    capture.now().wall,
                    Default::default(),
                ),
            );
            let screen = drawn(100, 32, &solo(&state), &mut ui, capture.now());
            let lines = band_lines(screen.buffer(), ui.shown.regions.composer);
            for delay in ["10s", "1m", "5m", "10m", "30m", "1h"] {
                assert!(
                    lines
                        .iter()
                        .any(|line| line.trim_start_matches([' ', '▎']) == format!("in {delay}"))
                );
            }
            assert_eq!(
                lines.iter().any(|line| line.trim() == "cancel the wake-up"),
                pending
            );
            let y = ui.shown.regions.composer.y
                + lines
                    .iter()
                    .position(|line| line.contains("▎ in 10s"))
                    .unwrap() as u16;
            assert_eq!(screen.buffer()[(3, y)].fg, crate::theme::text().fg.unwrap());
            assert_eq!(
                screen.buffer()[(1, y)].bg,
                crate::theme::palette().named("pick").unwrap()
            );
        }
    });
}
