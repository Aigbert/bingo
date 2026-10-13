//! Room I/O; records and folds live in bingo-collab.
use crate::identity;
pub use bingo_collab::room::*;
use bingo_sdk::{
    HostHandle, KernelError, OpenOptions, SessionFilter, SessionId, SessionSelector, SessionState,
};

/// The rooms hanging under one session, as the host lists its children.
/// Everything that asks "which rooms are here" — the listing, the reading at
/// the head of a turn, the door a verb names one at — asks it here.
pub async fn under(
    host: &HostHandle,
    parent: &SessionId,
) -> Result<Vec<(SessionId, Room)>, KernelError> {
    let children = host
        .sessions(SessionFilter {
            parent: Some(parent.clone()),
            ..SessionFilter::default()
        })
        .await?;
    Ok(children
        .into_iter()
        .filter_map(|child| Room::of(&child).map(|room| (child.id, room)))
        .collect())
}

/// A session as this plugin reads one: its own journal, folded. A session it
/// cannot read says nothing rather than guessing, which is what every caller
/// here wants of one.
pub async fn read(host: &HostHandle, session: &SessionId) -> Option<SessionState> {
    let opened = host
        .open(
            SessionSelector::ById {
                id: session.clone(),
            },
            identity(),
            OpenOptions::default(),
        )
        .await;
    match opened {
        Ok(attachment) => Some(attachment.snapshot),
        Err(error) => {
            tracing::debug!(%error, %session, "a session that cannot be read says nothing");
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ear::Ear;
    use crate::ear::{Ears, Seat};
    use crate::tests::{room_summary, summary};
    use bingo_sdk::View;
    use jiff::Timestamp;
    use serde_json::{Value, json};
    use std::time::Duration;

    #[test]
    fn a_log_session_keyed_rooms_under_a_parent_is_a_room() {
        let parent = SessionId::from_raw("ses_root");
        let room = Room::of(&room_summary("ses_design", &parent, "design")).expect("a room");
        assert_eq!(room.title, "#design");
        assert_eq!(room.parent, parent);
        assert!(room.members.is_empty(), "a summary says nothing of them");
        assert_eq!(room.ears, Ears::default(), "nor of what they hear");
        assert_eq!(room.purpose, None, "nor what it is for");
        assert!(!room.closed, "nor whether it still stands");
    }

    /// The shape an opening leaves in a journal — a fixture, because it is a
    /// persisted payload and not a value this process is free to change
    /// (ADR-0053 §1).
    #[test]
    fn an_opening_is_a_purpose_and_the_name_that_opened_it() {
        let opened = Opened {
            purpose: Some("settle the storage layout".into()),
            by: "reviewer".into(),
        };
        assert_eq!(
            opened.payload(),
            json!({"purpose": "settle the storage layout", "by": "reviewer"})
        );
        assert_eq!(
            Opened::of_payload(&opened.payload()),
            Some(opened),
            "and reads back as what it was written from"
        );

        const WRITTEN: &str = r#"{"purpose":"settle the storage layout","by":"reviewer"}"#;
        let payload: Value = serde_json::from_str(WRITTEN).expect("an opening payload");
        assert_eq!(
            Opened::of_payload(&payload).map(|opened| opened.by),
            Some("reviewer".to_string())
        );
    }

    /// A person's door may give no purpose (ADR-0053 §1), and the name that
    /// opened it is `parent`.
    #[test]
    fn an_opening_without_a_purpose_still_says_who_opened_it() {
        let opened = Opened {
            purpose: None,
            by: crate::name::PARENT.into(),
        };
        assert_eq!(opened.payload(), json!({"purpose": null, "by": "parent"}));
        assert_eq!(Opened::of_payload(&opened.payload()), Some(opened));
        assert_eq!(
            Opened::of_payload(&json!({"by": "parent"})),
            Some(Opened {
                purpose: None,
                by: "parent".into()
            }),
            "an absent purpose reads as no purpose"
        );
        assert_eq!(
            Opened::of_payload(&json!({"purpose": "x"})),
            None,
            "and a payload that names nobody says nothing"
        );
    }

    /// The whole of what closing a room means, as one value (ADR-0053 §4).
    #[test]
    fn a_closing_is_a_moment_a_name_and_what_was_said_about_it() {
        let closed = Closed {
            at: Timestamp::UNIX_EPOCH,
            by: "parent".into(),
            why: Some("it shipped".into()),
        };
        assert_eq!(
            closed.payload(),
            json!({"at": "1970-01-01T00:00:00Z", "by": "parent", "why": "it shipped"})
        );
        assert_eq!(Closed::of_payload(&closed.payload()), Some(closed));

        const WRITTEN: &str = r#"{"at":"1970-01-01T00:00:00Z","by":"parent","why":null}"#;
        let payload: Value = serde_json::from_str(WRITTEN).expect("a closing payload");
        assert_eq!(
            Closed::of_payload(&payload),
            Some(Closed {
                at: Timestamp::UNIX_EPOCH,
                by: "parent".into(),
                why: None,
            })
        );
    }

    /// The frame's presence is the whole of "closed": a payload this build
    /// cannot read closed the room, and only a null one — which nothing
    /// publishes — leaves it standing.
    #[test]
    fn a_room_is_closed_by_the_frame_and_not_by_what_is_in_it() {
        assert!(is_closed(Some(&json!({"at": "not a moment"}))));
        assert!(is_closed(Some(&json!({}))));
        assert!(!is_closed(None));
        assert!(!is_closed(Some(&Value::Null)));
        assert_eq!(
            Closed::of_payload(&json!({"at": "not a moment", "by": "parent"})),
            None,
            "and what it says is still unreadable"
        );
    }

    #[test]
    fn every_other_session_is_not_one() {
        let parent = SessionId::from_raw("ses_root");
        let agent = summary("ses_reviewer", Some("reviewer"), Some(parent.clone()));
        assert_eq!(Room::of(&agent), None, "a model answers in it");

        let mut untitled = room_summary("ses_x", &parent, "design");
        untitled.title = None;
        assert_eq!(Room::of(&untitled), None, "nothing to call it");

        let mut foreign = room_summary("ses_y", &parent, "design");
        foreign.key = Some("agent/ses_root/reviewer".into());
        assert_eq!(Room::of(&foreign), None, "another plugin minted it");

        let mut orphan = room_summary("ses_z", &parent, "design");
        orphan.parent = None;
        assert_eq!(Room::of(&orphan), None, "no tree to reach into");
    }

    #[test]
    fn a_membership_round_trips_through_the_payload_it_is_published_as() {
        let seats = [Seat::named("reviewer"), Seat::named("scout")];
        assert_eq!(members_from(&payload(&seats)), ["reviewer", "scout"]);
        assert!(members_from(&json!({})).is_empty());
        assert_eq!(
            members_from(&json!({ "members": ["reviewer", 7] })),
            ["reviewer"],
            "a name is a string"
        );
    }

    /// The shape a roster of bare names leaves in a journal — a fixture,
    /// because this is a persisted payload and not a value this process is free
    /// to change. It says nothing about ears, so every seat in it wears the
    /// default one, which is patient (ADR-0034 §6).
    #[test]
    fn a_roster_that_names_no_ear_reads_as_the_default_one() {
        const BARE: &str = r#"{"members":["reviewer","scout","parent"]}"#;
        let payload: Value = serde_json::from_str(BARE).expect("a membership payload");
        assert_eq!(members_from(&payload), ["reviewer", "scout", "parent"]);

        let mut ears = Ears::default();
        ears.declare(&payload);
        for member in ["reviewer", "scout", "parent"] {
            assert_eq!(ears.of(member), Ear::default(), "{member}");
        }
    }

    /// The whole of what a room's journal is handed, asserted as one value:
    /// the names a reader parses, the listeners' patience beside them and
    /// never instead of them, and the tree a surface draws (ADR-0013 §2).
    #[test]
    fn the_payload_carries_the_names_the_listeners_and_the_tree_it_draws_as() {
        assert_eq!(
            payload(&[Seat::named("scout")]),
            json!({
                "members": ["scout"],
                "kind": "tree",
                "nodes": [{"label": "scout", "tone": "neutral"}],
            }),
            "a roster of bare names says nothing about ears"
        );
        let asked = [
            Seat::live("scout"),
            Seat {
                name: "parent".into(),
                ear: Ear::Patient(Duration::from_secs(120)),
            },
        ];
        assert_eq!(
            payload(&asked),
            json!({
                "members": ["scout", "parent"],
                "listeners": [
                    {"name": "scout", "patience_s": 0},
                    {"name": "parent", "patience_s": 120},
                ],
                "kind": "tree",
                "nodes": [
                    {"label": "scout", "badge": "live", "tone": "neutral"},
                    {"label": "parent", "badge": "120s", "tone": "neutral"},
                ],
            })
        );
        assert_eq!(
            payload(&[])["nodes"],
            json!([{"label": "nobody yet", "tone": "neutral"}]),
            "a room nobody is in says so where a person looks"
        );
    }

    /// The tree rides beside the names, so a client that reads either sees the
    /// same roster and neither has to know about the other.
    #[test]
    fn the_payload_is_both_a_membership_and_a_view() {
        let seats = [Seat::named("reviewer"), Seat::named("scout")];
        let payload = payload(&seats);
        assert_eq!(members_from(&payload), ["reviewer", "scout"]);
        let view: View = serde_json::from_value(payload).expect("a view a surface can draw");
        assert_eq!(view.fold(), "reviewer\nscout");
    }
}
