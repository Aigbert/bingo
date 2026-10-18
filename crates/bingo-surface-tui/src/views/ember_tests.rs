use super::*;
use crate::test_support::{later, scene, still};
use bingo_sdk::view::{TextMotion, TextStyle};
use ratatui::{
    Terminal,
    backend::TestBackend,
    buffer::Buffer,
    style::{Color, Modifier},
    widgets::Paragraph,
};
use serde_json::json;

fn view(style: TextStyle) -> View {
    View::Text {
        children: vec![bingo_sdk::view::TextChild::String("ink".into())],
        style,
    }
}
fn draw(view: &View, now: Now) -> Buffer {
    let mut terminal = Terminal::new(TestBackend::new(12, 3)).unwrap();
    terminal
        .draw(|frame| {
            frame.render_widget(
                Paragraph::new(marked(view, 12, &Marks::at(now))),
                frame.area(),
            )
        })
        .unwrap();
    terminal.backend().buffer().clone()
}

#[test]
fn every_ember_ink_is_available_to_views() {
    theme::with(crate::painted::daylight(), || {
        for name in [
            "fg",
            "muted",
            "dim",
            "faint",
            "ember",
            "glow",
            "coal",
            "amber",
            "amberGlow",
            "good",
            "bad",
            "plan",
            "auto",
            "code",
            "crew0",
            "crew1",
            "crew2",
            "crew3",
            "crew4",
            "crew5",
            "kw",
            "str",
            "num",
            "fn",
            "ty",
            "ash",
            "presence",
            "#123456",
        ] {
            let buffer = draw(
                &view(TextStyle {
                    color: Some(name.into()),
                    ..Default::default()
                }),
                scene().1,
            );
            let expected = if name == "fg" {
                Color::Reset
            } else {
                theme::LIGHT.named(name).unwrap()
            };
            assert_eq!(buffer[(0, 0)].fg, expected, "{name}");
        }
    });
}

#[test]
fn every_ember_ground_fills_text_and_boxes() {
    theme::with(crate::painted::daylight(), || {
        for name in [
            "you",
            "well",
            "wellDeep",
            "pick",
            "pickBg",
            "addTint",
            "delTint",
            "addWord",
            "delWord",
            "badTint",
            "amberTint",
            "glowTint",
        ] {
            for kind in ["Text", "Box"] {
                let node = if kind == "Text" {
                    json!({"kind":kind,"ground":name,"children":["ink"]})
                } else {
                    json!({"kind":kind,"ground":name,"padding":1,"children":[{"kind":"Text","children":["ink"]}]})
                };
                let view: View = serde_json::from_value(node).unwrap();
                let buffer = draw(&view, scene().1);
                for x in 0..12 {
                    assert_eq!(
                        buffer[(x, 0)].bg,
                        theme::LIGHT.named(name).unwrap(),
                        "{kind} {name} x={x}"
                    );
                }
            }
        }
    });
}

#[test]
fn text_motion_uses_the_frame_clock_and_has_a_still_form() {
    theme::with(crate::painted::daylight(), || {
        let now = scene().1;
        let view = view(TextStyle {
            color: Some("glow".into()),
            motion: Some(Box::new(TextMotion::Breathe {
                with: "coal".into(),
                period_ms: 1800,
                depth: 1.0,
            })),
            ..Default::default()
        });
        assert_ne!(draw(&view, now), draw(&view, later(now, 900)));
        assert_eq!(draw(&view, still(now)), draw(&view, still(later(now, 900))));
        assert!(next_change(&view, now).is_some());
        assert!(next_change(&view, still(now)).is_none());
        let (_, deadline) = motion::track(now, || render(&view, 12));
        assert_eq!(deadline, next_change(&view, now));
    });
}

#[test]
fn fades_finish_and_nested_spinners_wake_on_their_own_beat() {
    theme::with(crate::painted::daylight(), || {
        let now = scene().1;
        let fade = view(TextStyle {
            color: Some("good".into()),
            motion: Some(Box::new(TextMotion::Fade {
                from: "glow".into(),
                at_ms: now.wall.as_millisecond(),
                ms: 420,
            })),
            ..Default::default()
        });
        assert_eq!(draw(&fade, now)[(0, 0)].fg, theme::LIGHT.glow);
        assert_eq!(draw(&fade, later(now, 420))[(0, 0)].fg, theme::LIGHT.good);
        assert!(next_change(&fade, later(now, 420)).is_none());
        let spin = view(TextStyle {
            motion: Some(Box::new(TextMotion::Spin)),
            ..Default::default()
        });
        let nested = View::Text {
            children: vec![bingo_sdk::view::TextChild::Node(Box::new(spin))],
            style: Default::default(),
        };
        assert_eq!(next_change(&nested, now), Some(crate::spinner::next(now)));
        assert_eq!(draw(&nested, still(now))[(0, 0)].symbol(), "◌");
        assert_ne!(
            draw(&nested, now),
            draw(
                &nested,
                later(now, crate::spinner::beat().as_millis() as i64)
            )
        );
    });
}

#[test]
fn unknown_names_are_plain_and_depth_conversion_keeps_modifiers() {
    for look in [
        crate::painted::daylight(),
        crate::painted::no_colour(),
        theme::Theme {
            colors: theme::Colors::Indexed(theme::LIGHT),
            glyphs: &theme::UNICODE,
        },
    ] {
        theme::with(look, || {
            assert_eq!(
                draw(
                    &view(TextStyle {
                        color: Some("unknown".into()),
                        ground: Some("unknown".into()),
                        ..Default::default()
                    }),
                    scene().1
                ),
                draw(&view(TextStyle::default()), scene().1)
            );
            let node = view(TextStyle {
                color: Some("dim".into()),
                ground: Some("pick".into()),
                bold: Some(true),
                italic: Some(true),
                underline: Some(true),
                ..Default::default()
            });
            let buffer = draw(&node, scene().1);
            let cell = &buffer[(0, 0)];
            assert!(
                cell.modifier
                    .contains(Modifier::BOLD | Modifier::ITALIC | Modifier::UNDERLINED)
            );
            match look.colors {
                theme::Colors::Plain => {
                    assert_eq!((cell.fg, cell.bg), (Color::Reset, Color::Reset));
                    assert!(cell.modifier.contains(Modifier::DIM | Modifier::REVERSED));
                }
                theme::Colors::Indexed(_) => {
                    assert!(matches!(cell.fg, Color::Indexed(_)));
                    assert!(matches!(cell.bg, Color::Indexed(_)));
                }
                _ => {}
            }
        });
    }
}

#[test]
fn a_cached_mod_row_animates_without_a_new_view_or_event() {
    use crate::test_support::*;
    use bingo_sdk::{Event, ItemId, slot::Slot};
    let state = folded(vec![
        frame(
            1,
            Event::ItemStarted {
                item: user("user", "hello"),
            },
        ),
        frame(
            2,
            extended(
                "fixture.ember",
                "row",
                bingo_sdk::slot::with(
                    json!({}),
                    [Slot::Row {
                        item: ItemId::from_raw("user"),
                        mark: None,
                        title: None,
                        badge: None,
                        caption: None,
                        actions: vec![],
                        body: Some(view(TextStyle {
                            color: Some("glow".into()),
                            motion: Some(Box::new(TextMotion::Breathe {
                                with: "coal".into(),
                                period_ms: 1800,
                                depth: 1.0,
                            })),
                            ..Default::default()
                        })),
                    }],
                ),
            ),
        ),
    ]);
    theme::with(crate::painted::daylight(), || {
        let (mut ui, now) = scene();
        draw_tree(100, 32, &solo(&state), &mut ui, now);
        let a = ui.shown.canvas.buffer.clone();
        assert!(ui.shown.next_frame.is_some());
        draw_tree(100, 32, &solo(&state), &mut ui, later(now, 900));
        assert_ne!(
            a, ui.shown.canvas.buffer,
            "the cached row must redraw on its deadline"
        );
    });
}

#[test]
fn box_grounds_stop_at_the_box_and_preserve_nested_grounds() {
    theme::with(crate::painted::daylight(), || {
        let view: View=serde_json::from_value(json!({"kind":"Box","width":6,"marginX":1,"ground":"you","children":[{"kind":"Text","ground":"well","children":["x"]}]})).unwrap();
        let buffer = draw(&view, scene().1);
        assert_eq!(buffer[(0, 0)].bg, Color::Reset);
        for x in 1..7 {
            assert_eq!(buffer[(x, 0)].bg, theme::LIGHT.well);
        }
        assert_eq!(buffer[(7, 0)].bg, Color::Reset);
    });
}
