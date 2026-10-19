use bingo_services::schedule::{Listing, Row};
use ratatui::{Terminal, backend::TestBackend, widgets::Paragraph};
fn listing(scene: &str) -> Listing {
    Listing {
        trouble: None,
        holder: if scene == "dormant" {
            "standby — another runner holds this store"
        } else {
            "held by this process (pid 4120)"
        }
        .into(),
        entries: if scene == "empty" {
            vec![]
        } else {
            vec![
                Row {
                    id: "01JA4".into(),
                    spec: "daily at 09:00".into(),
                    next_fire: Some("2026-10-16T01:00:00Z".into()),
                    enabled: true,
                    by: "you".into(),
                    mode: None,
                    text: "triage new issues".into(),
                    session: Some(bingo_sdk::SessionId::from_raw("ses_triage")),
                },
                Row {
                    id: "01JA7".into(),
                    spec: "every 2h".into(),
                    next_fire: Some("2026-10-15T11:37:00Z".into()),
                    enabled: true,
                    by: "model".into(),
                    mode: Some("acceptEdits".into()),
                    text: "/review dev\nthen post the summary to the team".into(),
                    session: None,
                },
                Row {
                    id: "01J9X".into(),
                    spec: "once at 2026-10-20T00:00:00Z".into(),
                    next_fire: None,
                    enabled: false,
                    by: "unconfirmed".into(),
                    mode: None,
                    text: "ship the notes".into(),
                    session: None,
                },
            ]
        },
    }
}
#[test]
fn schedules_at_both_sizes() {
    for (width, height) in [(108, 30), (80, 24)] {
        for scene in ["held", "dormant", "empty"] {
            let listing = listing(scene);
            let view = bingo_mod_schedule::draw::tree(
                &listing,
                listing.entries.get(1).map(|row| row.id.as_str()),
                &jiff::tz::TimeZone::UTC,
                width,
                height,
                "2026-10-15T11:00:00Z".parse().unwrap(),
                Default::default(),
            );
            let lines = crate::views::render(&view, usize::from(width));
            let mut terminal = Terminal::new(TestBackend::new(width, height)).unwrap();
            terminal
                .draw(|f| f.render_widget(Paragraph::new(lines), f.area()))
                .unwrap();
            let buffer = terminal.backend().buffer();
            let text = buffer
                .content
                .chunks(usize::from(width))
                .map(|r| {
                    r.iter()
                        .map(|c| c.symbol())
                        .collect::<String>()
                        .trim_end()
                        .to_string()
                })
                .collect::<Vec<_>>()
                .join("\n");
            insta::assert_snapshot!(format!("schedules_{scene}_{width}x{height}"), text);
            if scene != "empty" {
                assert!(text.contains("triage new issues"));
                assert!(text.contains("paused"));
            }
        }
    }
}

#[test]
fn hidden_schedule_text_survives_terminal_rendering_and_disarms_enter() {
    for (width, height) in [(80, 24), (108, 30)] {
        let mut listing = listing("ownership");
        listing.entries[1].text = "first\u{202e}\r\nsecond\u{1b}".into();
        let view = bingo_mod_schedule::draw::tree(
            &listing,
            listing.entries.get(1).map(|row| row.id.as_str()),
            &jiff::tz::TimeZone::UTC,
            width,
            height,
            jiff::Timestamp::UNIX_EPOCH,
            Default::default(),
        );
        let bingo_sdk::View::Box { layout, .. } = &view else {
            panic!("pane");
        };
        assert!(layout.on_key.as_ref().unwrap().args["confirming"].is_null());
        let lines = crate::views::render(&view, usize::from(width));
        let mut terminal = Terminal::new(TestBackend::new(width, height)).unwrap();
        terminal
            .draw(|f| f.render_widget(Paragraph::new(lines), f.area()))
            .unwrap();
        let text: String = terminal
            .backend()
            .buffer()
            .content
            .iter()
            .map(|c| c.symbol())
            .collect();
        assert!(text.contains("first\\u{202e}\\u{d}"));
        assert!(!text.contains('\u{202e}'));
        assert!(!text.contains('\u{1b}'));
        assert!(!text.contains("⏎ yes"));
    }
}
