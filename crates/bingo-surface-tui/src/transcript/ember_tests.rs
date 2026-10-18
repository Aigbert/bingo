use crate::{
    blocks::Blocks,
    test_support::ember::{self, Band, Capture},
    transcript::Rows,
};
use ratatui::{
    Terminal,
    backend::TestBackend,
    buffer::Buffer,
    widgets::{Paragraph, Widget},
};

fn draw(capture: &Capture, now: crate::clock::Now) -> Buffer {
    let (state, tools) = ember::story::scene(&capture.entry.file);
    let area = capture.area(Band::Stream);
    let folds = Default::default();
    let pictures = Default::default();
    let linked = Default::default();
    let mut rows = Rows::of(&state, area.width as usize, &folds, &pictures, &linked, now);
    rows.tools = tools;
    let mut blocks = Blocks::default();
    blocks.sync(&state, &Default::default(), &rows);
    let count = area.height as usize;
    let (lines, top, padding) = stream_window(&blocks, count);
    let mut terminal = Terminal::new(TestBackend::new(area.width, area.height)).unwrap();
    terminal
        .draw(|frame| frame.render_widget(Paragraph::new(lines), frame.area()))
        .unwrap();
    let mut buffer = terminal.backend().buffer().clone();
    if let Some((line, _)) =
        crate::transcript::sparks::anchor(&state).and_then(|id| blocks.span(id))
        && let Some(row) = line.checked_sub(top)
    {
        let drawing =
            crate::transcript::sparks::draw(&state, now, &mut buffer, area, (row + padding) as u16);
        if !drawing.lines.is_empty() {
            Paragraph::new(drawing.lines).render(area, &mut buffer);
        }
    }
    buffer
}

/// Captures include a breathing row and bottom-align their own stream area.
/// Only the block window API is shared with the full-frame renderer.
fn stream_window(
    blocks: &Blocks,
    count: usize,
) -> (Vec<ratatui::text::Line<'static>>, usize, usize) {
    let mut lines = blocks.window(0, blocks.height());
    lines.push(ratatui::text::Line::default());
    let top = lines.len().saturating_sub(count);
    let padding = count.saturating_sub(lines.len());
    lines.drain(..top);
    lines.splice(
        0..0,
        std::iter::repeat_n(ratatui::text::Line::default(), padding),
    );
    (lines, top, padding)
}

#[test]
fn ember_stream_bands_match_the_prototype() {
    for name in [
        "live",
        "done",
        "stop",
        "steer",
        "steer-landed",
        "ascii-live",
        "ascii-done",
        "reduced-live",
        "small-done",
        "narrow",
    ] {
        // The prototype phases each spinner from its step; the runtime shares a wall-clock beat.
        // Tolerate the glyph phase only in named mark cells; their inks still must match.
        let spinners: &[(u16, u16)] = match name {
            "live" | "ascii-live" => &[(25, 3)],
            "steer-landed" => &[(24, 3)],
            "narrow" => &[(17, 3)],
            _ => &[],
        };
        let capture = Capture::load(name).spinners(spinners);
        capture
            .render(|| ember::assert_band(&capture, Band::Stream, &draw(&capture, capture.now())));
    }
}

#[test]
fn ember_live_motion_stays_in_the_side_light_and_spinner() {
    let capture = Capture::load("live");
    capture.render(|| {
        // The capture is at 700 ms: sample 1400→1900 after the landing has settled.
        // Only the live band's side cell and the active step's spinner move; M193 owns the comet.
        let now = crate::test_support::later(capture.now(), 700);
        let before = draw(&capture, now);
        let after = draw(&capture, crate::test_support::later(now, 500));
        let moved = ember::moved(&before, &after);
        assert!(
            moved.contains(&(0, 19)) && moved.contains(&(3, 25)),
            "{moved:?}"
        );
        assert!(
            moved.iter().all(|cell| [(0, 19), (3, 25)].contains(cell)),
            "{moved:?}"
        );
    });
}
