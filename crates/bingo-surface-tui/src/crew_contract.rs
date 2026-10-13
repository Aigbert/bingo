//! M150's face and screen contracts, exercised before implementation.
use crate::test_support::*;
use crate::{face, painted, roster, status, theme};
use bingo_sdk::{Event, Level};
use ratatui::style::Color;

fn crew() -> crate::tree::Tree {
    let mut frames = busy_child("reviewer");
    frames.push(started_tool(
        1,
        tool(
            "itm_1",
            "SpawnAgent",
            serde_json::json!({"prompt":"review the manifest"}),
            None,
            bingo_sdk::ItemStatus::Running,
        ),
    ));
    frames.extend([
        agent_frame(3, 1, {
            let mut summary = agent_summary(3, "scout");
            summary.parent.as_mut().unwrap().item = None;
            Event::SessionUpdated { summary }
        }),
        agent_frame(3, 2, started("trn_scout")),
        log_frame(1, log_announced("#design")),
        log_frame(
            2,
            extended(
                "bingo.rooms",
                "members",
                roster_payload(&["parent", "reviewer", "scout"], &[("reviewer", 0)]),
            ),
        ),
        frame(
            20,
            signalled(
                "bingo.rooms",
                "owed",
                owed_payload(&[("#design", "scout", 2)]),
            ),
        ),
    ]);
    folded_tree(frames)
}

#[test]
fn faces_match_the_cross_surface_fixture() {
    let rows: serde_json::Value =
        serde_json::from_str(include_str!("../fixtures/faces.json")).unwrap();
    assert_eq!(rows.as_array().unwrap().len(), 20);
    for row in rows.as_array().unwrap() {
        let name = row["name"].as_str().unwrap();
        let got = face::face(name);
        assert_eq!(got.hash, row["hash"].as_u64().unwrap() as u32, "{name}");
        assert_eq!(got.hue, row["hue"].as_u64().unwrap() as usize);
        assert_eq!(got.sigil, row["sigil"].as_str().unwrap());
        theme::with(painted::ascii(), || {
            assert_eq!(face::face(name).sigil, row["ascii"].as_str().unwrap())
        });
    }
}

fn rgb(color: Color) -> [f64; 3] {
    let Color::Rgb(r, g, b) = color else {
        panic!("a truecolor face: {color:?}")
    };
    [r, g, b].map(|v| f64::from(v) / 255.0)
}

fn luminance(color: Color) -> f64 {
    let [r, g, b] = rgb(color).map(|c| {
        if c <= 0.04045 {
            c / 12.92
        } else {
            ((c + 0.055) / 1.055).powf(2.4)
        }
    });
    0.2126 * r + 0.7152 * g + 0.0722 * b
}

fn hue(color: Color) -> f64 {
    let [r, g, b] = rgb(color);
    let max = r.max(g).max(b);
    let min = r.min(g).min(b);
    let d = max - min;
    let h = if max == r {
        (g - b) / d
    } else if max == g {
        (b - r) / d + 2.0
    } else {
        (r - g) / d + 4.0
    };
    (h * 60.0).rem_euclid(360.0)
}

#[test]
fn crew_hues_are_readable_and_apart_from_state() {
    for look in [painted::truecolor(), painted::daylight()] {
        theme::with(look, || {
            let theme::Colors::True(p) = look.colors else {
                unreachable!()
            };
            for name in [
                "reviewer", "scout", "writer", "builder", "tester", "planner", "fox",
            ] {
                let color = face::style(name).fg.unwrap();
                let a = luminance(color);
                let b = luminance(p.raised);
                assert!(
                    (a.max(b) + 0.05) / (a.min(b) + 0.05) >= 4.5,
                    "{name} {color:?}"
                );
                for accent in [p.presence, p.good, p.bad, p.mode] {
                    let d = (hue(color) - hue(accent)).abs();
                    assert!(d.min(360.0 - d) >= 36.0, "{name}: {d}");
                }
            }
        });
    }
}

#[test]
fn child_row_has_a_face_and_keeps_lifecycle_words() {
    let (mut ui, now) = scene();
    let screen = draw_tree(120, 40, &crew(), &mut ui, now);
    assert!(
        screen.contains(&format!("{} reviewer(", face::face("reviewer").sigil)),
        "{screen}"
    );
    assert!(screen.contains("working · 3 tools"), "{screen}");
}

#[test]
fn roster_keeps_a_face_beside_the_live_ear() {
    let tree = crew();
    let list = roster::lines(
        &tree,
        &tree.rows(),
        roster::Cursor::default(),
        "",
        120,
        20,
        scene().1,
    );
    let row = list
        .lines
        .iter()
        .find(|line| line.to_string().contains("reviewer"))
        .unwrap()
        .to_string();
    assert!(row.contains(face::face("reviewer").sigil), "{row}");
    assert!(row.contains('~'), "{row}");
}

#[test]
fn strip_names_the_work_and_yields_to_notices() {
    let tree = crew();
    let (mut ui, now) = scene();
    let wide = status::line(&tree, &ui, 120, now).to_string();
    assert!(
        wide.contains("reviewer") && wide.contains("scout owes #design"),
        "{wide}"
    );
    assert!(!wide.contains("running"));
    let narrow = status::line(&tree, &ui, 80, now).to_string();
    assert!(
        !narrow.contains("reviewer") && narrow.contains(face::face("reviewer").sigil),
        "{narrow}"
    );
    ui.notify(Level::Info, "saved", now.instant);
    let notice = status::line(&tree, &ui, 120, later(now, 1000)).to_string();
    assert!(
        notice.contains("saved") && !notice.contains("reviewer"),
        "{notice}"
    );
    assert!(
        status::line(&tree, &ui, 120, later(now, 4001))
            .to_string()
            .contains("reviewer")
    );
}

fn chat() -> crate::tree::Tree {
    let mut tree = crew();
    tree.fold
        .apply(&posted(3, "itm_p1", "reviewer", "First post"));
    tree.fold.apply(&posted(
        4,
        "itm_p2",
        "reviewer",
        "@parent ask @scout in #design; @unknown stays plain",
    ));
    tree.show(&log_id());
    tree
}

#[test]
fn room_groups_authors_marks_the_reader_and_lists_members() {
    let (mut ui, now) = scene();
    let screen = draw_tree(120, 40, &chat(), &mut ui, now);
    assert_eq!(screen.matches("reviewer:").count(), 1, "{screen}");
    assert!(screen.contains("→ you"), "{screen}");
    assert!(
        screen.contains("members: parent, reviewer, scout"),
        "{screen}"
    );
}

#[test]
fn room_mentions_wear_the_known_names_hues() {
    theme::with(painted::truecolor(), || {
        let (mut ui, now) = scene();
        let screen = painted::painted(120, 40, &chat(), &mut ui, now);
        let row = screen.row("@parent ask");
        assert!(row.iter().any(|(text, style)| text.contains("@scout") && style.fg == face::style("scout").fg), "{row:?}");
        assert!(
            row.iter()
                .any(|(text, style)| text.contains("#design") && style.fg == theme::mode().fg),
            "{row:?}"
        );
    });
}
