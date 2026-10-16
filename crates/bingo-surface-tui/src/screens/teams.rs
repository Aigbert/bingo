//! The screens a team is read through (§3 "Teams"): a room's own transcript,
//! a post where a member reads it, what the three message tools answer, and
//! what a room is owed. They are `screens.rs`'s scenes and keep its snapshot
//! names — `both` is still the one that draws them.

use bingo_sdk::{ContentPart, Event, Item, ItemStatus, Tone, ToolOutput, TreeNode, View};
use serde_json::json;

use super::{both, item};

use crate::test_support::*;

/// A room reads as a conversation: the name, then the whole of what was said
/// under it. A post nobody signed is the holder's, and the roster's word for
/// that seat is what every member reads it under.
#[test]
fn a_room_transcript() {
    let tree = room_tree(vec![
        posted(
            2,
            "itm_1",
            "reviewer",
            "the plan is thin on tests\nnone of M9's exit criteria name one",
        ),
        posted(3, "itm_2", "scout", "M9's exit criteria cover them"),
        log_frame(
            4,
            Event::ItemCompleted {
                item: user("itm_3", "then let us ship it"),
            },
        ),
    ]);
    let (mut ui, now) = scene();
    let screen = draw_tree(80, 24, &tree, &mut ui, now);
    assert!(
        !screen.contains("⎿"),
        "no post is a folded result: {screen}"
    );
    assert!(screen.contains("✦ parent: then let us ship it"), "{screen}");
    both("room_transcript", &tree, &mut ui, now);
}

/// What a member's own transcript shows of the room it sits in: nothing
/// (ADR-0034). The wake that opened its turn, the reading that turn folded in,
/// and a post an older journal still carries are all the room's, and the room's
/// own view is one keystroke away. A message from a peer is not the room's, and
/// still arrives.
#[test]
fn a_room_post_in_a_member_s_own_transcript() {
    let state = folded(vec![
        item(
            1,
            voiced(
                delivered(
                    "itm_1",
                    "room",
                    None,
                    "#design has posts you have not read. Post in #design if any of \
                     it falls to you.",
                ),
                bingo_sdk::Voice::Aside,
            ),
        ),
        item(
            2,
            delivered(
                "itm_2",
                "contributor:rooms",
                None,
                "[#design, since you last read]\nreviewer: the plan is thin on tests",
            ),
        ),
        item(
            3,
            voiced(
                post("itm_3", "reviewer", "the plan is thin on tests"),
                bingo_sdk::Voice::Aside,
            ),
        ),
        item(
            4,
            delivered("itm_4", "agent", Some("scout"), "Two nits, else fine."),
        ),
    ]);
    let tree = solo(&state);
    let (mut ui, now) = scene();
    let screen = draw_tree(80, 24, &tree, &mut ui, now);
    assert!(
        !screen.contains("#design"),
        "not the nudge, not the reading, not the post: {screen}"
    );
    assert!(
        screen.contains("⏺ scout: Two nits"),
        "a message from a peer is not a room's: {screen}"
    );
    both("room_post", &tree, &mut ui, now);
}

/// A tool that drew for a person as well as answering the model.
fn answered_with(text: &str, view: View) -> Option<ToolOutput> {
    Some(ToolOutput {
        parts: vec![ContentPart::text(text)],
        is_error: false,
        display: Some(view),
        upshot: None,
    })
}

fn seat(label: &str, badge: Option<&str>, children: Vec<TreeNode>) -> TreeNode {
    TreeNode {
        label: label.into(),
        badge: badge.map(str::to_string),
        tone: Tone::Neutral,
        children,
    }
}

fn leaf(label: &str, badge: Option<&str>) -> TreeNode {
    seat(label, badge, Vec::new())
}

fn completed(id: &str, name: &str, input: serde_json::Value, out: Option<ToolOutput>) -> Item {
    tool(id, name, input, out, ItemStatus::Completed)
}

/// `OpenRoom`: the room and a node per seat, badged with the ear it asked for
/// where that is not the default one a bare name takes (ADR-0034 §6) — five
/// rows, which is exactly what a result keeps.
fn opened_a_room() -> Item {
    let seats = View::Tree {
        nodes: vec![seat(
            "#design",
            None,
            vec![
                leaf("helper", None),
                leaf("scout", None),
                leaf("watcher", Some("120s")),
                leaf("parent", Some("live")),
            ],
        )],
    };
    completed(
        "itm_1",
        "OpenRoom",
        json!({"name": "design", "members": ["helper", "scout"]}),
        answered_with("#design: helper, scout, watcher:120, parent:0", seats),
    )
}

/// `SendMessage`: where it went, whose name it arrives under, when it is read.
fn posted_to_a_room() -> Item {
    let receipt = View::KeyValue {
        rows: vec![
            ("to".into(), "#design".into()),
            ("from".into(), "parent".into()),
            (
                "read".into(),
                "at each seat's next turn; an @ wakes now".into(),
            ),
        ],
    };
    completed(
        "itm_2",
        "SendMessage",
        json!({"to": "#design", "text": "look again"}),
        answered_with("Posted to #design.", receipt),
    )
}

/// `ListAgents`: a node per agent, badged with what it is doing.
fn listed_the_agents() -> Item {
    let roster = View::Tree {
        nodes: vec![leaf("helper", Some("working")), leaf("scout", Some("idle"))],
    };
    completed(
        "itm_3",
        "ListAgents",
        json!({}),
        answered_with("helper  ses_2  working", roster),
    )
}

/// What the three tools a team is run through now answer (ADR-0013 §2). The
/// values are spelled here because a surface may not depend on a plugin —
/// what this asserts is the drawing, and `bingo-rooms` and `bingo-agents`
/// assert the values.
#[test]
fn what_the_message_tools_answer() {
    let state = folded(vec![
        item(1, opened_a_room()),
        item(2, posted_to_a_room()),
        item(3, listed_the_agents()),
    ]);
    let tree = solo(&state);
    let (mut ui, now) = scene();
    let wide = draw_tree(120, 40, &tree, &mut ui, now);
    assert!(wide.contains("├─ watcher [ 120s ]"), "{wide}");
    assert!(wide.contains("└─ parent [ live ]"), "{wide}");
    assert!(
        wide.contains("read  at each seat's next turn; an @ wakes now"),
        "{wide}"
    );
    assert!(wide.contains("├─ helper [ working ]"), "{wide}");
    both("message_tools", &tree, &mut ui, now);
}

// ---- the one list of sessions (M36) -------------------------------------

fn a_team() -> crate::tree::Tree {
    let mut frames = busy_child("reviewer");
    frames.extend([
        agent_frame(3, 20, agent_announced(3, "watcher")),
        agent_frame(4, 21, agent_announced(4, "scout")),
        agent_frame(4, 22, started("trn_4")),
        agent_frame(
            4,
            23,
            crate::test_support::completed("trn_4", bingo_sdk::TurnStatus::Completed),
        ),
        log_frame(30, log_announced("#design")),
        log_frame(
            31,
            extended(
                "bingo.rooms",
                "members",
                roster_payload(
                    &["reviewer", "watcher"],
                    &[("reviewer", 0), ("watcher", 600)],
                ),
            ),
        ),
        frame(
            32,
            signalled(
                "bingo.rooms",
                "owed",
                owed_payload(&[("#design", "reviewer", 22)]),
            ),
        ),
        item(33, user("itm_0", "what is in this workspace?")),
    ]);
    frames.extend((1..=4u64).map(|n| {
        posted(
            31 + n,
            &format!("itm_p{n}"),
            "watcher",
            &format!("post {n}"),
        )
    }));
    // Four posts stand in the room; this seat stopped reading at the first.
    frames.push(log_frame(36, room_cursor("reviewer", "itm_p1")));
    folded_tree(frames)
}

fn palette_team() -> crate::tree::Tree {
    let mut tree = a_team();
    let config = || bingo_sdk::ConfigView {
        kernel: json!({"render":{"Palette":[{}]}}),
        ..Default::default()
    };
    for frame in [
        frame(100, Event::ConfigChanged { config: config() }),
        child_frame(100, Event::ConfigChanged { config: config() }),
        log_frame(100, Event::ConfigChanged { config: config() }),
    ] {
        tree.fold.apply(&frame);
    }
    tree
}

#[tokio::test]
async fn the_palette_roster_preserves_status_unread_and_owed_columns() {
    let tree = palette_team();
    let fixture = crate::palette::tests::Fixture::new();
    let (mut ui, now) = scene();
    let now = still(now);
    crate::palette::open(&mut ui, bingo_sdk::component::PaletteOpen::Sessions, now);
    let wide = fixture.paint_tree(&mut ui, &tree, now, 160, 40).await;
    for text in [
        "this conversation",
        "reviewer",
        "working · in #design · 3 unread · live",
        "watcher",
        "listening · 600s",
        "owes an answer · 22m",
        "#design",
        "2 seats",
        "1 owed",
        "this one",
    ] {
        assert!(wide.contains(text), "missing {text}: {wide}");
    }
    insta::assert_snapshot!("palette_roster", wide);
    let painted = crate::painted::painted(160, 40, &tree, &mut ui, now);
    assert!(
        painted.coloured("this conversation").is_empty(),
        "group labels remain dim"
    );
    assert!(
        !painted.coloured("owes an answer · 22m").is_empty(),
        "debts retain attention colour"
    );
}

#[tokio::test]
async fn the_palette_roster_starts_on_the_viewed_room_and_returns_to_the_root() {
    let mut tree = palette_team();
    tree.show(&log_id());
    let fixture = crate::palette::tests::Fixture::new();
    let (mut ui, now) = scene();
    let now = still(now);
    crate::palette::open(&mut ui, bingo_sdk::component::PaletteOpen::Sessions, now);
    let screen = fixture.paint_tree(&mut ui, &tree, now, 80, 24).await;
    assert!(screen.contains("❯ #design"), "{screen}");
    insta::assert_snapshot!("palette_roster_in_the_room", screen);
    for _ in 0..4 {
        fixture
            .press_tree(&mut ui, &tree, key(crossterm::event::KeyCode::Up), now)
            .await;
        fixture.paint_tree(&mut ui, &tree, now, 80, 24).await;
    }
    let effects = fixture
        .press_tree(&mut ui, &tree, key(crossterm::event::KeyCode::Enter), now)
        .await;
    assert_eq!(
        effects,
        vec![crate::effect::Effect::View(tree.root_id().clone())]
    );
}

#[tokio::test]
async fn the_palette_roster_has_identical_keyboard_and_crew_doors() {
    use crossterm::event::{KeyModifiers, MouseButton, MouseEvent, MouseEventKind};
    let tree = palette_team();
    let fixture = crate::palette::tests::Fixture::new();
    let (mut keyboard, now) = scene();
    let (mut pointer, _) = scene();
    let now = still(now);
    crate::input::on_key(&mut keyboard, &tree, ctrl('r'), now);
    draw_tree(120, 40, &tree, &mut pointer, now);
    let area = pointer.shown.crew.unwrap();
    crate::pointer::on_mouse(
        &mut pointer,
        &tree,
        MouseEvent {
            kind: MouseEventKind::Down(MouseButton::Left),
            column: area.x,
            row: area.y,
            modifiers: KeyModifiers::NONE,
        },
        now,
    );
    for (width, height) in [(80, 24), (120, 40)] {
        assert_eq!(
            fixture
                .paint_tree(&mut keyboard, &tree, now, width, height)
                .await,
            fixture
                .paint_tree(&mut pointer, &tree, now, width, height)
                .await
        );
    }
}
