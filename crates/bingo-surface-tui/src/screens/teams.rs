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
            delivered(
                "itm_1",
                "room",
                None,
                "#design has posts you have not read. Post in #design if any of \
                 it falls to you.",
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
        item(3, post("itm_3", "reviewer", "the plan is thin on tests")),
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
