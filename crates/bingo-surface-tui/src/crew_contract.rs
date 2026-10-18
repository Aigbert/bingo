//! M150's face and screen contracts, exercised before implementation.
use crate::test_support::*;
use crate::{face, footer, painted, theme};
use bingo_sdk::{Event, Level};

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
        assert_eq!(got.0, row["hash"].as_u64().unwrap() as u32, "{name}");
        assert_eq!(got.hue(), row["hue"].as_u64().unwrap() as usize);
        assert_eq!(got.sigil(), row["sigil"].as_str().unwrap());
        theme::with(painted::ascii(), || {
            assert_eq!(face::face(name).sigil(), row["ascii"].as_str().unwrap())
        });
    }
}

#[test]
fn crew_hues_use_the_prototype_identity_palette() {
    for look in [painted::truecolor(), painted::daylight()] {
        theme::with(look, || {
            let theme::Colors::True(p) = look.colors else {
                unreachable!()
            };
            for name in [
                "reviewer", "scout", "writer", "builder", "tester", "planner", "fox", "Scout",
            ] {
                let color = face::style(name).fg.unwrap();
                assert!(p.crew().contains(&color), "{name}: {color:?}");
            }
        });
    }
}

#[test]
fn child_row_has_a_face_and_keeps_lifecycle_words() {
    let (mut ui, now) = scene();
    let screen = draw_tree(120, 40, &crew(), &mut ui, now);
    assert!(
        screen.contains(&format!("{} reviewer(", face::face("reviewer").sigil())),
        "{screen}"
    );
    assert!(screen.contains("working · 3 tools"), "{screen}");
}

#[test]
fn strip_names_the_work_and_yields_to_notices() {
    let tree = crew();
    let (mut ui, now) = scene();
    for width in [80, 120] {
        let wide =
            footer::draw(&tree, &ui, &mut Default::default(), width, now).lines[0].to_string();
        assert!(wide.contains("●● 2 agents"), "{wide}");
        assert!(!wide.contains("reviewer"));
    }
    ui.notify(Level::Info, "saved", now.instant);
    let notice = footer::draw(&tree, &ui, &mut Default::default(), 120, later(now, 1000)).lines[0]
        .to_string();
    assert!(notice.contains("saved") && !notice.contains("2 agents"));
    assert!(
        footer::draw(&tree, &ui, &mut Default::default(), 120, later(now, 4133)).lines[0]
            .to_string()
            .contains("2 agents")
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
fn a_child_permission_and_room_posts_keep_the_selected_room_in_front() {
    let mut tree = crew();
    let (mut ui, now) = scene();
    tree.show(&log_id());
    assert_eq!(
        crate::focus::owner(&ui, &tree),
        crate::focus::Owner::Composer
    );
    let steps = [
        child_frame(20, opened(child_permission())),
        child_frame(
            21,
            Event::InteractionResolved {
                id: child_permission().id,
                answer: bingo_sdk::Answer::AllowOnce,
                by: bingo_sdk::ResolvedBy::Kernel,
            },
        ),
        child_frame(22, completed("trn_9", bingo_sdk::TurnStatus::Completed)),
        child_frame(23, started("trn_next")),
        posted(3, "itm_p1", "reviewer", "First review post"),
        posted(4, "itm_p2", "reviewer", "@parent consult @scout in #design"),
        child_frame(24, completed("trn_next", bingo_sdk::TurnStatus::Completed)),
    ];
    for (step, event) in steps.iter().enumerate() {
        assert_ne!(tree.fold.apply(event), bingo_sdk::Applied::Stale);
        ui.dialog
            .focus_on(tree.open_interaction().map(|(_, ask)| ask));
        assert_eq!(tree.view(), &log_id(), "step {step}");
        let screen = draw_tree(120, 40, &tree, &mut ui, now);
        assert!(screen.contains("Post to #design"), "step {step}: {screen}");
        assert_eq!(tree.open_interaction().is_some(), step == 0);
        assert_eq!(
            crate::focus::owner(&ui, &tree),
            if step == 0 {
                crate::focus::Owner::Interaction
            } else {
                crate::focus::Owner::Composer
            },
            "step {step}"
        );
        if step >= 4 {
            assert!(screen.contains("First review post"), "{screen}");
        }
    }
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

#[test]
fn live_faces_pulse_through_the_cache_and_stillness_preserves_words() {
    for look in [
        painted::truecolor(),
        painted::daylight(),
        painted::no_colour(),
        painted::ascii(),
    ] {
        theme::with(look, || {
            let tree = crew();
            let (mut ui, now) = scene();
            let before = painted::painted(120, 40, &tree, &mut ui, now).row("reviewer(");
            let after =
                painted::painted(120, 40, &tree, &mut ui, later(now, 1000)).row("reviewer(");
            assert_ne!(before, after);
            let stopped = painted::painted(120, 40, &tree, &mut ui, still(now)).row("reviewer(");
            assert_eq!(
                stopped,
                painted::painted(120, 40, &tree, &mut ui, still(later(now, 1000))).row("reviewer(")
            );
        });
    }
}

#[test]
fn strip_scopes_overflows_finishes_and_clicks() {
    let mut tree = crew();
    let (mut ui, now) = scene();
    ui.catalogs
        .commands
        .push(crate::commands::entry(bingo_sdk::CommandSpec {
            opens: false,
            name: "resume".into(),
            keys: vec!["ctrl+r".into()],
            aliases: vec![],
            hint: String::new(),
            args: bingo_sdk::ArgSpec::None,
            instant: true,
            hidden: false,
            family: "session".into(),
        }));
    draw_tree(120, 40, &tree, &mut ui, now);
    tree.fold.apply(&child_frame(
        20,
        completed("trn_9", bingo_sdk::TurnStatus::Completed),
    ));
    assert!(
        !footer::draw(&tree, &scene().0, &mut Default::default(), 120, now).lines[0]
            .to_string()
            .contains("reviewer")
    );
    tree.show(&child_id());
    assert!(
        crate::crew::active(&tree).is_empty(),
        "neither parent nor siblings belong to this strip"
    );
}

#[test]
fn a_placed_status_outranks_crew_until_it_expires() {
    let mut tree = crew();
    let (mut ui, now) = scene();
    tree.fold.apply(&frame(
        40,
        signalled(
            "example.status",
            "status",
            bingo_sdk::slot::with(
                serde_json::json!({}),
                [bingo_sdk::slot::Slot::Status {
                    text: "scheduled".into(),
                    tone: bingo_sdk::Tone::Neutral,
                    until: Some(now.wall + jiff::SignedDuration::from_secs(2)),
                }],
            ),
        ),
    ));
    let placed = footer::draw(&tree, &ui, &mut Default::default(), 120, now).lines[0].to_string();
    assert!(placed.contains("scheduled") && !placed.contains("reviewer"));
    draw_tree(120, 40, &tree, &mut ui, now);
    assert!(
        footer::draw(&tree, &ui, &mut Default::default(), 120, later(now, 2001)).lines[0]
            .to_string()
            .contains("2 agents")
    );
}

#[test]
fn prose_only_colours_known_mentions_and_refreshes_when_names_arrive() {
    theme::with(painted::truecolor(), || {
        let mut tree = crew();
        let names = crate::markdown::mentions::known(&tree);
        let lines =
            crate::markdown::mentioned("@SCOUT #design @unknown mail@scout `@scout`", 120, &names)
                .lines;
        let spans = &lines[0].spans;
        assert!(
            spans
                .iter()
                .any(|span| span.content == "@SCOUT" && span.style.fg == face::style("scout").fg)
        );
        assert!(
            spans
                .iter()
                .filter(|span| span.content.contains("unknown")
                    || span.content.contains("mail@")
                    || span.content.contains('`'))
                .all(|span| span.style.fg != face::style("scout").fg)
        );
        tree.fold.apply(&frame(
            40,
            Event::ItemCompleted {
                item: assistant("itm_a", "Hello @newcomer", bingo_sdk::ItemStatus::Completed),
            },
        ));
        let (mut ui, now) = scene();
        let before = painted::painted(120, 40, &tree, &mut ui, now).row("Hello @newcomer");
        tree.fold
            .apply(&agent_frame(5, 1, agent_announced(5, "newcomer")));
        let after = painted::painted(120, 40, &tree, &mut ui, now).row("Hello @newcomer");
        assert_ne!(
            before, after,
            "an unchanged item takes the newly known name's hue"
        );
    });
}

#[test]
fn room_members_refresh_and_the_composer_stays_in_place() {
    let mut tree = chat();
    let (mut ui, now) = scene();
    let before = draw_tree(120, 40, &tree, &mut ui, now);
    let composer = ui.shown.regions.composer;
    tree.fold.apply(&log_frame(
        5,
        extended(
            "bingo.rooms",
            "members",
            roster_payload(&["parent", "reviewer"], &[]),
        ),
    ));
    let after = draw_tree(120, 40, &tree, &mut ui, now);
    assert_ne!(before, after);
    assert!(after.contains("members: parent, reviewer"));
    assert!(!after.contains("members: parent, reviewer, scout"));
    assert_eq!(composer, ui.shown.regions.composer);
}
