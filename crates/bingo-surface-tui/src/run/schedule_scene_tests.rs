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
        for scene in ["held", "dormant", "empty", "confirming", "ownership"] {
            let listing = listing(scene);
            let confirming = match scene {
                "confirming" => Some(bingo_mod_schedule::Confirmation::Forget(
                    listing.entries[1].clone(),
                )),
                "ownership" => Some(bingo_mod_schedule::Confirmation::Confirm(
                    listing.entries[1].clone(),
                )),
                _ => None,
            };
            let view = bingo_mod_schedule::draw::tree(
                &listing,
                1,
                confirming.as_ref(),
                width,
                height,
                "2026-10-15T11:00:00Z".parse().unwrap(),
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
            if !matches!(scene, "empty" | "ownership") {
                let modifier = |id: &str| {
                    buffer
                        .content
                        .chunks(usize::from(width))
                        .find(|row| {
                            row.iter()
                                .map(|c| c.symbol())
                                .collect::<String>()
                                .contains(id)
                        })
                        .unwrap()[2]
                        .modifier
                };
                assert_ne!(modifier("01JA4"), modifier("01J9X"));
            }
        }
    }
}

#[test]
fn hidden_schedule_text_survives_terminal_rendering_and_disarms_enter() {
    for (width, height) in [(80, 24), (108, 30)] {
        let mut listing = listing("ownership");
        listing.entries[1].text = "first\u{202e}\r\nsecond\u{1b}".into();
        let confirmation = bingo_mod_schedule::Confirmation::Confirm(listing.entries[1].clone());
        let view = bingo_mod_schedule::draw::tree(
            &listing,
            1,
            Some(&confirmation),
            width,
            height,
            jiff::Timestamp::UNIX_EPOCH,
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
        assert!(text.contains("second\\u{1b}"));
        assert!(text.contains("/schedule confirm 01JA7"));
        assert!(!text.contains("⏎ yes"));
    }
}
