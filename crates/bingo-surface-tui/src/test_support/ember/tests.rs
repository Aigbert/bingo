use super::*;
use ratatui::{
    Terminal,
    backend::TestBackend,
    style::Style,
    text::{Line, Span},
    widgets::Paragraph,
};

fn done_dock() -> Buffer {
    let mut terminal = Terminal::new(TestBackend::new(100, 3)).unwrap();
    terminal
        .draw(|frame| {
            frame.render_widget(
                Paragraph::new(vec![
                    Line::default(),
                    Line::from(vec![
                        Span::raw("   "),
                        Span::raw("▍"),
                        Span::styled("Reply, or / for commands", theme::dim()),
                    ]),
                    Line::default(),
                ])
                .style(theme::ground("you").unwrap()),
                frame.area(),
            );
        })
        .unwrap();
    terminal.backend().buffer().clone()
}

#[test]
fn three_hand_built_done_rows_pass_and_each_failure_explains_its_cell() {
    let capture = Capture::load("done");
    capture.render(|| {
        let buffer = done_dock();
        assert_band(&capture, Band::Dock, &buffer);
        let mut changed = buffer.clone();
        changed[(10, 1)].set_symbol("!");
        let report = compare::compare(&capture, Band::Dock, &changed, changed.area).unwrap_err();
        assert!(
            report.contains("first difference at cell (10, 1): character"),
            "{report}"
        );
        assert!(report.contains("capture") && report.contains("rendered") && report.contains('^'));
        assert!(report.contains("capture ink: bg") && report.contains("rendered ink:"));
        assert!(std::panic::catch_unwind(|| assert_band(&capture, Band::Dock, &changed)).is_err());
        let mut changed = buffer.clone();
        changed[(10, 1)].set_style(theme::bad());
        let report = compare::compare(&capture, Band::Dock, &changed, changed.area).unwrap_err();
        assert!(report.contains("fg: expected dim"), "{report}");
        assert_eq!(moved(&buffer, &changed), vec![(10, 1)]);
        assert!(std::panic::catch_unwind(|| assert_band(&capture, Band::Dock, &changed)).is_err());
    });
}

#[test]
fn every_capture_loads_its_geometry_metadata_and_ink_runs() {
    let root =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../docs/design/ember/scenes/index.json");
    let entries: Vec<Entry> =
        serde_json::from_str(&std::fs::read_to_string(root).unwrap()).unwrap();
    for entry in entries {
        let capture = Capture::load(&entry.file);
        assert_eq!(capture.area(Band::Frame), capture.size());
        assert_eq!(
            capture.area(Band::Stream).height
                + capture.area(Band::Dock).height
                + capture.area(Band::Footer).height,
            capture.size().height
        );
        assert_eq!(
            capture.now().wall.as_millisecond(),
            CLOCK_ORIGIN_MS + entry.moment_ms
        );
    }
}

#[test]
fn wide_cells_reset_inks_and_spinner_sets_compare_as_terminal_cells() {
    assert!(
        compare::compare(
            &Capture::load("done"),
            Band::Dock,
            &Buffer::empty(Rect::new(0, 0, 1, 1)),
            Rect::new(0, 0, 1, 1)
        )
        .is_err()
    );
    let rows = parse::rows(
        "# fixture\n你⠋ \n",
        "00 fg 0-1 fg · 2 glow\n00 bg 0-1 you · 2-3 bg",
    );
    let capture = Capture {
        entry: Entry {
            file: "fixture".into(),
            scene: "fixture".into(),
            group: String::new(),
            moment_ms: 0,
            size: "4x1".into(),
            flags: serde_json::Value::Null,
            after: vec![],
        },
        rows,
        spinner_cells: vec![(0, 2)],
    };
    let mut buffer = Buffer::empty(Rect::new(0, 0, 4, 1));
    buffer.set_string(0, 0, "你", Style::new().bg(theme::LIGHT.you()));
    buffer[(1, 0)].reset();
    buffer.set_string(2, 0, "⠼", Style::new().fg(theme::LIGHT.glow()));
    assert_band(&capture, Band::Frame, &buffer);
}

#[test]
fn ordinary_punctuation_is_not_a_spinner() {
    let mut capture = Capture::load("done");
    capture.rows = parse::rows("# fixture\nbingo-core\n", "");
    capture.spinner_cells.clear();
    let mut buffer = Buffer::empty(Rect::new(0, 0, 10, 1));
    buffer.set_string(0, 0, "bingo|core", Style::default());
    assert!(compare::compare(&capture, Band::Frame, &buffer, buffer.area).is_err());
    capture = capture.spinners(&[(0, 5)]);
    assert_band(&capture, Band::Frame, &buffer);
    buffer[(5, 0)].set_symbol("⠋");
    assert!(compare::compare(&capture, Band::Frame, &buffer, buffer.area).is_err());
}

#[test]
fn stream_regions_include_activity_and_strip() {
    let mut capture = Capture::load("done");
    capture.rows = parse::rows("# fixture\na\nb\nc\nf\n", "");
    let mut buffer = Buffer::empty(Rect::new(0, 0, 1, 4));
    for (y, ch) in ["a", "b", "c", "f"].into_iter().enumerate() {
        buffer[(0, y as u16)].set_symbol(ch);
    }
    let regions = crate::frame::Regions {
        transcript: Rect::new(0, 0, 1, 1),
        activity: Rect::new(0, 1, 1, 1),
        strip: Rect::new(0, 2, 1, 1),
        ..Default::default()
    };
    assert_regions(&capture, Band::Stream, &buffer, &regions);
}
