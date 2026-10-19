use super::ember_tests::fixture;
use crate::test_support::{
    ember::{Band, Capture, assert_regions},
    *,
};
use bingo_sdk::*;

fn emit(state: &mut SessionState, at: jiff::Timestamp, event: Event) {
    let mut frame = frame(state.seq.0 + 1, event);
    frame.ts = at;
    state.apply(&frame);
}
#[test]
fn m198_sessions_switched_full_frame() {
    let capture = Capture::load("sessions-switched");
    capture.render(|| {
        let (previous, mut ui) = fixture(&capture);
        let mut state = state();
        let mut summary = previous.summary;
        summary.title = Some("windows CI 超时".into());
        emit(
            &mut state,
            capture.now().wall,
            Event::SessionUpdated { summary },
        );
        let start = capture.now().wall - jiff::SignedDuration::from_secs(10_000);
        crate::test_support::ember::story::script(
            &mut state,
            "pastScript",
            "windows CI 超时",
            start,
            1300,
        );
        ui.notify(
            Level::Info,
            "✓ resumed · windows CI 超时",
            capture.now().instant - std::time::Duration::from_millis(500),
        );
        let screen = drawn(100, 32, &solo(&state), &mut ui, capture.now());
        assert_regions(&capture, Band::Frame, screen.buffer(), &ui.shown.regions);
    });
}
