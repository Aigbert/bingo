//! The rooms this process has seen. Every entry is a fold of a frame the hook
//! observed — a room enters because its own `SessionUpdated` said what it is,
//! and its membership changes because an `Extension` frame said so — so the
//! roster is never a second source for either (ADR-0011 §2).

use std::collections::BTreeMap;
use std::sync::{Mutex, MutexGuard};

use bingo_sdk::{SessionId, SessionSummary};
use serde_json::Value;

use crate::room::{self, Room};

#[derive(Debug, Default)]
pub struct Roster(Mutex<BTreeMap<SessionId, Entry>>);

#[derive(Debug)]
struct Entry {
    room: Room,
    seen_live: bool,
}

impl Roster {
    /// A room announcing itself, at the head of its stream. A reopen says the
    /// same thing again and must not forget its folded metadata. Hand it back
    /// only on the first live announcement, even if discovery restored it.
    pub fn register(&self, summary: &SessionSummary) -> Option<Room> {
        let room = Room::of(summary)?;
        let mut rooms = self.rooms();
        let entry = rooms.entry(summary.id.clone()).or_insert(Entry {
            room,
            seen_live: false,
        });
        if entry.seen_live {
            return None;
        }
        entry.seen_live = true;
        Some(entry.room.clone())
    }

    /// A background read must not overwrite a room already observed live.
    /// Arm under the roster lock so a live closing cannot precede stale timers.
    pub(crate) fn restore(&self, state: &bingo_sdk::SessionState, arm: impl FnOnce(&Room)) {
        let Some(room) = Room::of(&state.summary).map(|room| room.seated(state)) else {
            return;
        };
        let mut rooms = self.rooms();
        if rooms.contains_key(&state.summary.id) {
            return;
        }
        arm(&room);
        rooms.insert(
            state.summary.id.clone(),
            Entry {
                room,
                seen_live: false,
            },
        );
    }

    /// The rooms this process has seen under one session: everyone its card
    /// speaks for.
    pub fn under(&self, parent: &SessionId) -> Vec<(SessionId, Room)> {
        self.rooms()
            .iter()
            .filter(|(_, entry)| &entry.room.parent == parent)
            .map(|(id, entry)| (id.clone(), entry.room.clone()))
            .collect()
    }

    /// One of this plugin's frames in a room's journal: the whole of its
    /// membership, one seat's own ear, what it was opened for, or that it has
    /// ended. Nothing else it publishes is a room's business.
    pub fn extended(&self, session: &SessionId, kind: &str, payload: &Value) {
        let mut rooms = self.rooms();
        let Some(Entry { room, .. }) = rooms.get_mut(session) else {
            return;
        };
        match kind.strip_prefix(room::EAR) {
            Some(member) => room.ears.retune(member, payload),
            None if kind == room::MEMBERS => {
                room.members = room::members_from(payload);
                room.ears.declare(payload);
            }
            None if kind == room::OPENED => {
                room.purpose = room::Opened::of_payload(payload).and_then(|opened| opened.purpose)
            }
            None if kind == room::CLOSED => room.closed = room::is_closed(Some(payload)),
            None => {}
        }
    }

    /// The room a session is, for a caller that is about to await: a copy, so
    /// no lock is held across one.
    pub fn get(&self, session: &SessionId) -> Option<Room> {
        self.rooms().get(session).map(|entry| entry.room.clone())
    }

    fn rooms(&self) -> MutexGuard<'_, BTreeMap<SessionId, Entry>> {
        self.0.lock().unwrap_or_else(|held| held.into_inner())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ear::{self, Ear, Seat};
    use crate::room::{MEMBERS, payload};
    use crate::tests::{room_summary, summary};

    fn members(names: [&str; 2]) -> Value {
        payload(&names.map(Seat::live))
    }

    #[test]
    fn a_restored_room_is_handed_back_once_when_first_seen_live() {
        let fleet = crate::tests::Fleet::default();
        let parent = fleet.root();
        let id = fleet.room(&parent, "design");
        fleet.remember(&id, crate::tests::extension(members(["reviewer", "scout"])));
        let snapshot = fleet.state(&id);
        let roster = Roster::default();
        roster.restore(&snapshot, |_| {});

        let room = roster
            .register(&snapshot.summary)
            .expect("first live announcement");
        assert_eq!(room.members, ["reviewer", "scout"]);
        assert_eq!(roster.register(&snapshot.summary), None);
        assert_eq!(roster.get(&id), Some(room));
    }

    #[test]
    fn a_late_background_snapshot_does_not_replace_a_live_roster_or_its_timers() {
        let fleet = crate::tests::Fleet::default();
        let parent = fleet.root();
        let id = fleet.room(&parent, "design");
        let snapshot = fleet.state(&id);
        let roster = Roster::default();
        roster.register(&snapshot.summary);
        roster.extended(&id, MEMBERS, &members(["reviewer", "scout"]));
        roster.restore(&snapshot, |_| panic!("a live room already owns its timers"));
        assert_eq!(roster.get(&id).unwrap().members, ["reviewer", "scout"]);
    }

    #[test]
    fn a_room_enters_when_it_announces_itself_and_a_reopen_changes_nothing() {
        let parent = SessionId::from_raw("ses_root");
        let announced = room_summary("ses_design", &parent, "design");
        let roster = Roster::default();

        assert!(roster.register(&announced).is_some(), "a room this new");
        roster.extended(&announced.id, MEMBERS, &members(["reviewer", "scout"]));
        assert_eq!(roster.register(&announced), None, "and not again");

        let room = roster.get(&announced.id).expect("still one room");
        assert_eq!(room.title, "#design");
        assert_eq!(room.members, ["reviewer", "scout"]);
        assert_eq!(roster.rooms().len(), 1, "a reopen is the same room");
        assert_eq!(roster.under(&parent), [(announced.id, room)]);
        assert!(roster.under(&SessionId::from_raw("ses_other")).is_empty());
    }

    /// The two frames a room's ears come in, folded by the one arm that reads
    /// them: the roster declares, and a seat retunes its own.
    #[test]
    fn a_seat_s_own_ear_is_folded_beside_the_roster_that_declared_it() {
        let parent = SessionId::from_raw("ses_root");
        let announced = room_summary("ses_design", &parent, "design");
        let roster = Roster::default();
        roster.register(&announced);

        roster.extended(&announced.id, MEMBERS, &members(["reviewer", "scout"]));
        roster.extended(
            &announced.id,
            &ear::kind("scout"),
            &ear::register(Ear::Patient(ear::FLOOR)),
        );

        let room = roster.get(&announced.id).expect("the room");
        assert_eq!(room.ears.of("scout"), Ear::Patient(ear::FLOOR));
        assert_eq!(
            room.ears.of("reviewer"),
            Ear::Live,
            "as the roster seated it"
        );
    }

    /// The other two frames a room's own journal carries (ADR-0053 §1, §4),
    /// folded by the same arm: what it is for, and that it has ended.
    #[test]
    fn a_room_folds_what_it_was_opened_for_and_that_it_has_closed() {
        let parent = SessionId::from_raw("ses_root");
        let announced = room_summary("ses_design", &parent, "design");
        let roster = Roster::default();
        roster.register(&announced);

        let room = roster.get(&announced.id).expect("the room");
        assert_eq!(room.purpose, None, "an announce says nothing of either");
        assert!(!room.closed);

        roster.extended(
            &announced.id,
            room::OPENED,
            &room::Opened {
                purpose: Some("settle the storage layout".into()),
                by: "parent".into(),
            }
            .payload(),
        );
        let room = roster.get(&announced.id).expect("the room");
        assert_eq!(room.purpose.as_deref(), Some("settle the storage layout"));
        assert!(!room.closed, "a room that was opened still stands");

        roster.extended(
            &announced.id,
            room::CLOSED,
            &room::Closed {
                at: jiff::Timestamp::UNIX_EPOCH,
                by: "parent".into(),
                why: None,
            }
            .payload(),
        );
        assert!(roster.get(&announced.id).expect("the room").closed);
    }

    #[test]
    fn nothing_that_is_not_a_room_enters_and_nothing_unknown_takes_members() {
        let parent = SessionId::from_raw("ses_root");
        let agent = summary("ses_reviewer", Some("reviewer"), Some(parent));
        let roster = Roster::default();

        roster.register(&agent);
        roster.extended(&agent.id, MEMBERS, &members(["a", "b"]));
        assert_eq!(roster.get(&agent.id), None);
    }
}
