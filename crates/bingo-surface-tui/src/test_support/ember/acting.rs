use super::Capture;

/// The scenes that act on a live turn: their keys typed into a run whose
/// turn the timeline streams.
const ACTING: &[&str] = &[
    "live-steered",
    "live-stopped",
    "stop",
    "stop-resent",
    "steer",
    "steer-landed",
    "shell",
    "shell-ls",
    "image",
    "image-sent",
];

/// The capture `name`, its keys pressed into a run, compared full frame.
pub(crate) fn full_frame(name: &str) {
    if ACTING.contains(&name) {
        acting_frame(name);
        return;
    }
    let spinners: &[(u16, u16)] = match name {
        "ask-edit-allowed" | "question-answered" => &[(25, 3)],
        "tasks" | "tasks-again" => &[(25, 3), (26, 3)],
        "schedule-ran" => &[(26, 3)],
        _ => &[],
    };
    sheet_frame(name, spinners);
}

fn acting_frame(name: &str) {
    let spinners: &[(u16, u16)] = match name {
        "live-steered" => &[(25, 3)],
        "steer-landed" => &[(24, 3)],
        _ => &[],
    };
    let capture = Capture::load(name).spinners(spinners);
    capture.render(|| {
        tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap()
            .block_on(crate::run::acting::draw(&capture));
    });
}

/// Each sheet capture, its keys pressed into a run whose sheets the owners draw.
fn sheet_frame(name: &str, spinners: &[(u16, u16)]) {
    let capture = Capture::load(name).spinners(spinners);
    capture.render(|| {
        let buffer = pressed(&capture);
        super::assert_band(&capture, super::Band::Frame, &buffer);
    });
}

fn pressed(capture: &Capture) -> ratatui::buffer::Buffer {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap()
        .block_on(crate::run::acting::draw_sheet(capture))
}
