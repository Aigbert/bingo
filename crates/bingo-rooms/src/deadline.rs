//! Owned patience deadlines derived from room cursors.
use crate::cursor::Unread;
use crate::ear::Ear;
use crate::post;
use crate::room::{self, Room};
use bingo_alarm::{Alarm, Alarms};
use bingo_sdk::{HostHandle, SessionId};
use std::sync::{Mutex, MutexGuard};
use std::time::Duration;

type Waiting = (SessionId, SessionId);
#[derive(Debug, Default)]
pub struct Deadline {
    waiting: Mutex<Alarms<Waiting>>,
}
impl Deadline {
    pub async fn waiting(&self, host: &HostHandle, id: &SessionId, room: &Room, seats: &[&str]) {
        for member in seats {
            let Ear::Patient(patience) = room.ears.of(member) else {
                continue;
            };
            let Some(seat) = post::seat_of(host, room, member).await else {
                continue;
            };
            self.arm(host, id, room, member, seat, patience);
        }
    }

    pub async fn reconcile(
        &self,
        host: &HostHandle,
        id: &SessionId,
        room: &Room,
        now: jiff::Timestamp,
    ) {
        let Some(state) = room::read(host, id).await else {
            return;
        };
        let mut wanted = Vec::new();
        if !room.closed {
            for member in &room.members {
                let Ear::Patient(patience) = room.ears.of(member) else {
                    continue;
                };
                let unread = Unread::of(&state, member);
                let Some(first) = unread.posts.first() else {
                    continue;
                };
                let Some(seat) = post::seat_of(host, room, member).await else {
                    continue;
                };
                wanted.push((seat.clone(), id.clone()));
                let left = patience.saturating_sub(crate::chase::waited(first.at, now));
                self.arm(host, id, room, member, seat, left);
            }
        }
        self.timed()
            .retain(|key| &key.1 != id || wanted.contains(key));
    }

    #[cfg(test)]
    pub async fn overdue(&self, host: &HostHandle, id: &SessionId, room: &Room) {
        self.reconcile(host, id, room, jiff::Timestamp::now()).await;
    }

    fn arm(
        &self,
        host: &HostHandle,
        id: &SessionId,
        room: &Room,
        member: &str,
        seat: SessionId,
        patience: Duration,
    ) {
        let key = (seat.clone(), id.clone());
        let mut alarms = self.timed();
        if alarms.contains(&key) {
            return;
        }
        let (host, id, room, member) = (host.clone(), id.clone(), room.clone(), member.to_owned());
        alarms.arm(
            key,
            Alarm::new(tokio::time::Instant::now() + patience, async move {
                wake_if_behind(&host, &id, &room, &member, &seat).await;
            }),
        );
    }

    pub fn clear(&self) {
        self.timed().clear();
    }
    fn timed(&self) -> MutexGuard<'_, Alarms<Waiting>> {
        self.waiting.lock().unwrap_or_else(|held| held.into_inner())
    }
}

/// A seat behind its room, woken for it. Nothing is said of what it missed —
/// the turn this opens reads the room itself.
async fn wake_if_behind(
    host: &HostHandle,
    id: &SessionId,
    room: &Room,
    member: &str,
    seat: &SessionId,
) {
    let Some(state) = room::read(host, id).await else {
        return;
    };
    if room::closed_of(&state) || Unread::of(&state, member).is_empty() {
        return;
    }
    post::nudge(host, seat, &room.title, post::unread(&room.title)).await;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cursor;
    use crate::ear::{Ears, Seat};
    use crate::room::{self, MEMBERS};
    use crate::roster::Roster;
    use crate::tests::{Fleet, ts};
    use bingo_sdk::{Delivery, Input};

    /// The seats a roster line asks for.
    fn seats(roster: &[&str]) -> Vec<Seat> {
        roster
            .iter()
            .map(|word| Seat::read(word).expect("a roster word"))
            .collect()
    }

    /// A root with a scout under it and a room the roster seats them both in,
    /// as the frames this process saw would have left it.
    fn tree(roster: &[&str]) -> (Fleet, SessionId, SessionId, SessionId, Room) {
        let fleet = Fleet::default();
        let root = fleet.root();
        let scout = fleet.child(&root, "scout");
        let id = fleet.room(&root, "design");
        let rooms = Roster::default();
        rooms.register(&fleet.summary(&id));
        rooms.extended(&id, MEMBERS, &room::payload(&seats(roster)));
        let room = rooms.get(&id).expect("the room this process saw");
        (fleet, root, scout, id, room)
    }

    /// The room as a reader that never saw a frame holds it.
    fn room_of(parent: &SessionId, roster: &[&str]) -> Room {
        let seated = seats(roster);
        let mut ears = Ears::default();
        ears.declare(&room::payload(&seated));
        Room {
            title: "#design".into(),
            parent: parent.clone(),
            members: seated.into_iter().map(|seat| seat.name).collect(),
            ears,
            purpose: None,
            closed: false,
        }
    }

    fn nudges(fleet: &Fleet) -> Vec<(SessionId, String)> {
        fleet
            .delivered()
            .into_iter()
            .filter_map(|(to, input, delivery)| match input {
                Input::Text { text, origin, .. } if origin.principal.is_none() => {
                    assert_eq!(delivery, Delivery::Wake, "a nudge wakes the seat");
                    Some((to, text))
                }
                _ => None,
            })
            .collect()
    }

    /// One patience at a time, the chaser's own wait: a task that has not run
    /// yet has not asked for its sleep.
    async fn after(patience: Duration) {
        settle().await;
        tokio::time::advance(patience).await;
        settle().await;
    }

    async fn settle() {
        for _ in 0..8 {
            tokio::task::yield_now().await;
        }
    }

    /// A post into the room, and the seat's cursor moved to it.
    fn said(fleet: &Fleet, id: &SessionId, text: &str) {
        fleet.post(id, text, Some("reviewer"), ts());
    }

    async fn read_it_all(fleet: &Fleet, id: &SessionId) {
        let head = room::read(&fleet.handle(), id)
            .await
            .expect("the room")
            .transcript
            .items()
            .last()
            .expect("a post")
            .id
            .clone();
        cursor::advance(&fleet.handle(), id, "scout", &head)
            .await
            .expect("a cursor this crate can write");
    }

    /// The whole of ADR-0029 §3 on the paused clock: a seat a post left behind
    /// is woken once when it has been behind its whole patience, and not again
    /// while it is behind on the same posts.
    #[tokio::test(start_paused = true)]
    async fn a_seat_left_behind_is_woken_once_when_its_patience_is_up() {
        let (fleet, _, scout, id, room) = tree(&["scout:120"]);
        let deadline = Deadline::default();
        said(&fleet, &id, "the build is green");

        deadline
            .waiting(&fleet.handle(), &id, &room, &["scout"])
            .await;
        after(Duration::from_secs(119)).await;
        assert!(
            nudges(&fleet).is_empty(),
            "nothing before the patience is up"
        );

        after(Duration::from_secs(1)).await;
        let nudged = nudges(&fleet);
        assert_eq!(nudged.len(), 1, "{nudged:?}");
        assert_eq!(nudged[0].0, scout);
        assert!(nudged[0].1.contains("#design"), "{}", nudged[0].1);
    }

    /// The storm the ADR was written against, bounded: four patient members and
    /// one poster. Nobody is woken before the patience is up, each is woken
    /// once when it is, and a member the post names is woken at once whatever
    /// its ear (ADR-0029 §5).
    #[tokio::test(start_paused = true)]
    async fn four_patient_members_and_one_poster_cost_one_wake_each_per_patience() {
        let roster = ["alpha:120", "beta:120", "gamma:120", "delta:120"];
        let fleet = Fleet::default();
        let root = fleet.root();
        for member in ["alpha", "beta", "gamma", "delta"] {
            fleet.child(&root, member);
        }
        let id = fleet.room(&root, "design");
        let rooms = Roster::default();
        rooms.register(&fleet.summary(&id));
        rooms.extended(&id, MEMBERS, &room::payload(&seats(&roster)));
        let room = rooms.get(&id).expect("the room this process saw");
        let deadline = Deadline::default();
        let host = fleet.handle();

        let waiting = post::fan_out(&host, &room, "reviewer", "the build is green")
            .await
            .expect("a post this crate can fan out");
        assert_eq!(waiting, ["alpha", "beta", "gamma", "delta"], "none woken");
        deadline.waiting(&host, &id, &room, &waiting).await;
        said(&fleet, &id, "the build is green");

        after(Duration::from_secs(119)).await;
        assert!(nudges(&fleet).is_empty(), "{:?}", nudges(&fleet));

        after(Duration::from_secs(1)).await;
        let nudged = nudges(&fleet);
        assert_eq!(nudged.len(), 4, "one wake each, and only at the deadline");
        after(Duration::from_secs(600)).await;
        assert_eq!(nudges(&fleet).len(), 4, "and not again while it stands");

        let named = post::fan_out(&host, &room, "reviewer", "@alpha does it ship?")
            .await
            .expect("a post");
        assert_eq!(
            named,
            ["beta", "gamma", "delta"],
            "the named seat was woken"
        );
        assert_eq!(nudges(&fleet).len(), 5);
    }

    /// A second post does not start the patience over: one timer per seat per
    /// room, so a room that keeps talking is still bounded.
    #[tokio::test(start_paused = true)]
    async fn a_later_post_does_not_move_the_deadline_the_first_one_set() {
        let (fleet, _, _, id, room) = tree(&["scout:120"]);
        let deadline = Deadline::default();
        let host = fleet.handle();
        said(&fleet, &id, "the build is green");
        deadline.waiting(&host, &id, &room, &["scout"]).await;

        after(Duration::from_secs(60)).await;
        said(&fleet, &id, "and the tests pass");
        deadline.waiting(&host, &id, &room, &["scout"]).await;

        after(Duration::from_secs(60)).await;
        assert_eq!(nudges(&fleet).len(), 1, "the first post's own deadline");
    }

    /// The seat read the room before the deadline came due: the timer wakes,
    /// finds its cursor at the head, and says nothing.
    #[tokio::test(start_paused = true)]
    async fn a_seat_that_reads_the_room_before_the_deadline_is_not_nudged() {
        let (fleet, _, _, id, room) = tree(&["scout:120"]);
        let deadline = Deadline::default();
        let host = fleet.handle();
        said(&fleet, &id, "the build is green");
        deadline.waiting(&host, &id, &room, &["scout"]).await;

        after(Duration::from_secs(60)).await;
        read_it_all(&fleet, &id).await;

        after(Duration::from_secs(600)).await;
        assert!(nudges(&fleet).is_empty(), "{:?}", fleet.delivered());
    }

    /// A live seat was woken by the post itself; nothing here is armed for it.
    #[tokio::test(start_paused = true)]
    async fn a_live_seat_is_nobody_s_deadline() {
        let (fleet, _, _, id, room) = tree(&["scout:0"]);
        let deadline = Deadline::default();
        said(&fleet, &id, "the build is green");
        deadline
            .waiting(&fleet.handle(), &id, &room, &["scout"])
            .await;

        after(Duration::from_secs(3600)).await;
        assert!(fleet.delivered().is_empty(), "{:?}", fleet.delivered());
    }

    /// The holder is a seat like any other: its patience is read off the
    /// roster under the name the room calls it by.
    #[tokio::test(start_paused = true)]
    async fn the_session_the_room_hangs_under_is_a_patient_seat_too() {
        let (fleet, root, _, id, room) = tree(&["scout:0", "parent:120"]);
        let deadline = Deadline::default();
        said(&fleet, &id, "the build is green");
        deadline
            .waiting(&fleet.handle(), &id, &room, &["parent"])
            .await;

        after(Duration::from_secs(120)).await;
        let nudged = nudges(&fleet);
        assert_eq!(nudged.len(), 1, "{nudged:?}");
        assert_eq!(nudged[0].0, root);
    }

    /// A room this process reads for the first time may find a seat already
    /// behind — the cursor the last process left. It waited out a patience
    /// nobody was timing, so it is nudged once, and at once.
    #[tokio::test(start_paused = true)]
    async fn a_seat_this_process_finds_already_behind_is_nudged_once() {
        let (fleet, root, scout, id, _) = tree(&["scout:120"]);
        said(&fleet, &id, "the build is green");
        let room = room_of(&root, &["scout:120"]);
        let deadline = Deadline::default();

        deadline.overdue(&fleet.handle(), &id, &room).await;
        settle().await;
        let nudged = nudges(&fleet);
        assert_eq!(nudged.len(), 1, "{nudged:?}");
        assert_eq!(nudged[0].0, scout);

        read_it_all(&fleet, &id).await;
        deadline.overdue(&fleet.handle(), &id, &room).await;
        settle().await;
        assert_eq!(
            nudges(&fleet).len(),
            1,
            "a seat that has read the room is left alone"
        );
    }

    /// A live seat is never behind: whatever it has not read, it has not read
    /// because it is busy, and a busy seat reads at its next round.
    #[tokio::test(start_paused = true)]
    async fn a_live_seat_s_backlog_is_nobody_s_deadline() {
        let (fleet, root, _, id, _) = tree(&["scout:0"]);
        said(&fleet, &id, "the build is green");
        Deadline::default()
            .overdue(&fleet.handle(), &id, &room_of(&root, &["scout:0"]))
            .await;
        settle().await;
        assert!(fleet.delivered().is_empty(), "{:?}", fleet.delivered());
    }

    /// A seat seated at the room's head is level with it, so nothing wakes it
    /// for what was said before it took its place (ADR-0034 §2).
    #[tokio::test(start_paused = true)]
    async fn a_seat_seated_at_the_head_starts_level() {
        let (fleet, root, scout, id, _) = tree(&["scout:120"]);
        said(&fleet, &id, "said before you were seated");
        read_it_all(&fleet, &id).await;

        Deadline::default()
            .overdue(&fleet.handle(), &id, &room_of(&root, &["scout:120"]))
            .await;
        settle().await;
        assert!(fleet.delivered().is_empty(), "{:?}", fleet.delivered());
        assert_ne!(scout, id, "the seat and its room are two sessions");
    }

    /// A cursor at the head is the whole of "read": no post of the room is
    /// unread, whoever wrote it.
    #[tokio::test]
    async fn a_cursor_at_the_head_leaves_nothing_unread() {
        let (fleet, _, _, id, _) = tree(&["scout:120"]);
        said(&fleet, &id, "the build is green");
        let host = fleet.handle();
        let read = |fleet: &Fleet| Unread::of(&fleet.state(&id), "scout");
        assert!(!read(&fleet).is_empty());

        read_it_all(&fleet, &id).await;
        let unread = read(&fleet);
        assert!(unread.is_empty(), "{unread:?}");
        assert_eq!(unread.head, None, "and nothing left to move the cursor to");
        assert!(
            cursor::of_state(&room::read(&host, &id).await.expect("the room"), "scout").is_some(),
            "the cursor is a register in the room's own journal"
        );
    }
}
