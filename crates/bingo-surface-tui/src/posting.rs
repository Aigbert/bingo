//! A line typed into a room (`submitInput`, `post`): refused while the room
//! is closed, and once it goes, told who it called on.
use bingo_collab::mentions::named;
use bingo_collab::room::is_room;
use bingo_sdk::toast::Say;
use bingo_sdk::{SessionState, pick, view::TextChild};

use crate::tree::Tree;

/// Why a line does not go: a closed room takes no post. A command still runs.
pub fn refused(state: &SessionState, text: &str) -> Option<Say> {
    let closed = is_room(&state.summary) && bingo_collab::room::closed_of(state);
    (closed && !command(text)).then(|| Say::new(["this room is closed"]))
}

/// What a post into the room on screen is told: a teammate it names who is
/// not seated, or that it called on nobody.
pub fn told(tree: &Tree, text: &str) -> Option<Say> {
    let room = tree.viewed();
    if !is_room(&room.summary) || command(text) || text.trim().is_empty() {
        return None;
    }
    let seated = crate::seats::members(room);
    if let Some(away) = away(tree, text, &seated) {
        return Some(Say::new([
            TextChild::from(format!("{away} is not in this room")),
            TextChild::from(pick::text(format!("/seat {away}"), "glow")),
            TextChild::from("brings them in"),
        ]));
    }
    named(text, &seated)
        .is_empty()
        .then(|| Say::new(["posted", "@ someone to ask for an answer"]))
}

/// The first teammate of the tree the line names who has no seat here.
fn away(tree: &Tree, text: &str, seated: &[String]) -> Option<String> {
    let crew: Vec<String> = crate::face::Faces::of(tree)
        .teammates()
        .map(|row| bingo_collab::name::of_session(row).to_owned())
        .filter(|name| {
            !seated
                .iter()
                .any(|held| bingo_collab::name::same(held, name))
        })
        .collect();
    named(text, &crew).into_iter().find_map(|owed| match owed {
        bingo_collab::mentions::Owed::Member(name) => Some(name),
        bingo_collab::mentions::Owed::Room => None,
    })
}

fn command(text: &str) -> bool {
    text.trim_start().starts_with('/')
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::{
        agent_frame, agent_id, agent_summary, child_frame, child_id, child_summary, log_frame,
        room_tree, roster_payload,
    };

    fn room(members: &[&str]) -> Tree {
        let mut frames = vec![log_frame(
            2,
            crate::test_support::extended("bingo.rooms", "members", roster_payload(members, &[])),
        )];
        frames.push(child_frame(
            3,
            bingo_sdk::Event::SessionUpdated {
                summary: child_summary("scout"),
            },
        ));
        room_tree(frames)
    }

    #[test]
    fn a_closed_room_refuses_a_post_and_still_runs_a_command() {
        let mut tree = room(&["scout"]);
        let closed = bingo_collab::room::Closed {
            at: crate::test_support::ts(),
            by: "parent".into(),
            why: None,
        };
        tree.fold.apply(&log_frame(
            4,
            crate::test_support::extended("bingo.rooms", "closed", closed.payload()),
        ));
        let said = refused(tree.viewed(), "anyone there?").expect("a closed room");
        assert_eq!(said.plain(), "this room is closed");
        assert!(refused(tree.viewed(), "/room").is_none());
        assert!(refused(room(&["scout"]).viewed(), "hello").is_none());
    }

    #[test]
    fn a_post_names_who_is_away_or_says_it_called_nobody() {
        let tree = room(&[]);
        let said = told(&tree, "@scout look").expect("scout is not seated");
        assert_eq!(
            said.plain(),
            "scout is not in this room · /seat scout · brings them in"
        );
        let said = told(&tree, "just a note").expect("nobody was called");
        assert_eq!(said.plain(), "posted · @ someone to ask for an answer");
        assert!(told(&room(&["scout"]), "@scout look").is_none());
        assert!(told(&tree, "/seat scout").is_none());
    }

    /// `away` and the inks read one crew: a teammate's own hire is not
    /// home's teammate, so naming it is not naming someone away.
    #[test]
    fn a_teammates_own_hire_is_neither_away_nor_crew_ink() {
        let mut tree = room(&[]);
        let hire = bingo_sdk::SessionSummary {
            parent: Some(bingo_sdk::ParentLink {
                session: child_id(),
                item: None,
            }),
            ..agent_summary(7, "helper")
        };
        tree.fold.apply(&agent_frame(
            7,
            4,
            bingo_sdk::Event::SessionUpdated { summary: hire },
        ));
        assert!(
            tree.state(&agent_id(7)).is_some(),
            "the hire is in the tree"
        );
        let faces = crate::face::Faces::of(&tree);
        assert_eq!(faces.ink(&agent_id(7)), "muted");
        assert!(faces.ink(&child_id()).starts_with("crew"));
        let said = told(&tree, "@helper look").expect("nobody seated was called");
        assert_eq!(said.plain(), "posted · @ someone to ask for an answer");
        let said = told(&tree, "@scout look").expect("scout is home's teammate");
        assert_eq!(
            said.plain(),
            "scout is not in this room · /seat scout · brings them in"
        );
    }
}
