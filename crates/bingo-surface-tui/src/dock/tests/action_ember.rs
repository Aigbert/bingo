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

#[test]
fn m198_schedule_ran_full_frame() {
    let capture = Capture::load("schedule-ran").spinners(&[(26, 3)]);
    capture.render(|| {
        let (mut state, mut ui) = fixture(&capture);
        // The capture advances 60 ms after Enter, then waits 400 ms.
        let start = capture.now().wall - jiff::SignedDuration::from_millis(460);
        let rows = crate::test_support::ember::world::rows("scheds");
        crate::test_support::ember::story::script(
            &mut state,
            "echoScript",
            rows[1]["what"].as_str().unwrap(),
            start,
            460,
        );
        ui.notify(
            Level::Info,
            format!("paused · {}", rows[0]["what"].as_str().unwrap()),
            capture.now().instant - std::time::Duration::from_millis(880),
        );
        let screen = drawn(100, 32, &solo(&state), &mut ui, capture.now());
        assert_regions(&capture, Band::Frame, screen.buffer(), &ui.shown.regions);
    });
}
