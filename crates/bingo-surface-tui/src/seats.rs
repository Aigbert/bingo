//! The room columns a session carries in the switcher.
use crate::tree::{self, Tree};
use bingo_collab::lifecycle::Lifecycle;
use bingo_collab::name::same;
pub use bingo_collab::room::{HOLDER, Hearing as Ear, Owes, members_of as members};
use bingo_collab::room::{hearing as ear, unread_count as unread};
use bingo_sdk::SessionState;

/// Where a session sits, as the room it sits in has it.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Seat {
    /// The room's name, `#design`.
    pub room: String,
    pub ear: Ear,
    /// How many posts stand after this seat's reading mark (ADR-0034 §2),
    /// where there is a mark to read and it is behind the room's head. A seat
    /// at the head has nothing to say, and so has one the room's journal
    /// carries no mark for that this surface recognises — a guess would be
    /// worse than a silence.
    pub unread: Option<u64>,
    /// The oldest answer it still owes there, where one stands.
    pub owes: Option<Owes>,
}

/// What a room's own row says: how many seats it has, and how many of them
/// owe an answer. A member with two debts is one debtor, as `/room` says it.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[cfg(test)]
pub struct Counts {
    pub seats: usize,
    pub owed: usize,
}

/// The seat a session holds, or nothing at all for one no room has seated.
/// A session sits in at most one room here: the first that names it, in the
/// tree's own order.
pub fn seat(tree: &Tree, state: &SessionState) -> Option<Seat> {
    let name = tree::name(state);
    let room = rooms(tree).find(|room| members(room).iter().any(|held| same(held, &name)))?;
    Some(Seat {
        room: tree::name(room),
        ear: ear(room, &name),
        unread: unread(room, &name),
        owes: owes(tree, room, &name),
    })
}

/// What a room's row says about itself.
#[cfg(test)]
pub fn counts(tree: &Tree, room: &SessionState) -> Counts {
    let title = tree::name(room);
    let mut debtors: Vec<String> = Vec::new();
    for (who, _) in debts(tree, room, &title) {
        if !debtors.iter().any(|held| same(held, &who)) {
            debtors.push(who);
        }
    }
    Counts {
        seats: members(room).len(),
        owed: debtors.len(),
    }
}

/// The sessions of the tree that answer nobody: a room is a `Log` session, and
/// that is the same fact the list groups its two runs by.
fn rooms(tree: &Tree) -> impl Iterator<Item = &SessionState> {
    tree.sessions()
        .filter(|state| Lifecycle::of_state(state).is_none())
}

/// The oldest answer this seat still owes. The debts are published oldest
/// first, so the first of them for a name is the one to say.
fn owes(tree: &Tree, room: &SessionState, name: &str) -> Option<Owes> {
    debts(tree, room, &tree::name(room))
        .into_iter()
        .find(|(who, _)| same(who, name))
        .map(|(_, owes)| owes)
}

/// The open debts of one room: who owes, and what a row can say about when.
/// They are signalled onto the room's *parent*, which is where a person looks
/// (ADR-0022 §4), so this reaches across the tree for them.
fn debts(tree: &Tree, room: &SessionState, title: &str) -> Vec<(String, Owes)> {
    let parent = room.summary.parent.as_ref().map(|link| &link.session);
    parent
        .and_then(|id| tree.sessions().find(|state| &state.summary.id == id))
        .map(|parent| bingo_collab::room::debts(parent, title))
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::*;

    /// A root with a room under it, two sub-agents seated in it — one that
    /// asked for a live ear, one wearing a patience of its own — four posts in
    /// the room, and a debt the room's parent is signalling.
    fn seated() -> Tree {
        let mut frames = vec![
            child_frame(1, announced("reviewer")),
            agent_frame(3, 2, agent_announced(3, "watcher")),
            log_frame(3, log_announced("#design")),
            log_frame(
                4,
                extended(
                    "bingo.rooms",
                    "members",
                    roster_payload(
                        &["reviewer", "watcher"],
                        &[("reviewer", 0), ("watcher", 600)],
                    ),
                ),
            ),
        ];
        // The posts a reading mark has to stand behind.
        frames.extend(
            (1..=4u64)
                .map(|n| posted(4 + n, &format!("itm_p{n}"), "watcher", &format!("post {n}"))),
        );
        frames.push(frame(
            5,
            signalled(
                "bingo.rooms",
                "owed",
                owed_payload(&[("#design", "reviewer", 22)]),
            ),
        ));
        folded_tree(frames)
    }

    /// The moment a debt `minutes` old was asked at, against the clock every
    /// scene is drawn with.
    fn asked_minutes_ago(minutes: i64) -> Owes {
        Owes::Since(ts() - jiff::SignedDuration::from_mins(minutes))
    }

    fn of(tree: &Tree, name: &str) -> Option<Seat> {
        let state = tree
            .sessions()
            .find(|state| tree::name(state) == name)
            .expect("a session by that name");
        seat(tree, state)
    }

    #[test]
    fn a_member_sits_in_the_room_whose_roster_names_it() {
        let tree = seated();
        assert_eq!(
            of(&tree, "reviewer"),
            Some(Seat {
                room: "#design".into(),
                ear: Ear::Live,
                unread: None,
                owes: Some(asked_minutes_ago(22)),
            })
        );
        assert_eq!(of(&tree, "project"), None, "the root sits in no room");
    }

    #[test]
    fn a_listener_wears_the_patience_the_roster_declared_for_it() {
        assert_eq!(
            of(&seated(), "watcher").map(|seat| seat.ear),
            Some(Ear::Listening {
                patience_s: Some(600)
            })
        );
    }

    /// The two layers of ADR-0029 §4: a seat's own register outranks what the
    /// roster declared, and a cleared one gives the declaration back.
    #[test]
    fn a_seat_that_retuned_its_own_ear_is_heard_over_the_roster() {
        let mut tree = seated();
        tree.fold.apply(&log_frame(
            9,
            extended(
                "bingo.rooms",
                "ear:watcher",
                serde_json::json!({"patience_s": 60}),
            ),
        ));
        assert_eq!(
            of(&tree, "watcher").map(|seat| seat.ear),
            Some(Ear::Listening {
                patience_s: Some(60)
            })
        );

        tree.fold.apply(&log_frame(
            10,
            extended(
                "bingo.rooms",
                "ear:watcher",
                serde_json::json!({"patience_s": 0}),
            ),
        ));
        assert_eq!(
            of(&tree, "watcher").map(|seat| seat.ear),
            Some(Ear::Live),
            "no patience at all is the live seat it describes"
        );
    }

    /// The shape a room opened before there were ears left in its journal, and
    /// the shape a bare name still writes: a roster names only the seats whose
    /// ear is not the default, and the default is the patient one now
    /// (ADR-0034 §6).
    #[test]
    fn a_roster_that_declares_no_ear_seats_everyone_patient() {
        let tree = folded_tree(vec![
            child_frame(1, announced("reviewer")),
            log_frame(2, log_announced("#design")),
            log_frame(
                3,
                extended(
                    "bingo.rooms",
                    "members",
                    serde_json::json!({"members": ["reviewer"]}),
                ),
            ),
        ]);
        assert_eq!(
            of(&tree, "reviewer"),
            Some(Seat {
                room: "#design".into(),
                ear: Ear::Listening { patience_s: None },
                unread: None,
                owes: None,
            })
        );
    }

    /// The one fact "seen" derives from (ADR-0034 §2): the posts of the room's
    /// own journal that landed after the one this seat's mark points at, and
    /// nothing at all once the mark has caught up.
    #[test]
    fn a_seat_behind_the_rooms_head_counts_what_it_has_not_read() {
        let mut tree = seated();
        tree.fold
            .apply(&log_frame(9, room_cursor("reviewer", "itm_p1")));
        assert_eq!(
            of(&tree, "reviewer").and_then(|seat| seat.unread),
            Some(3),
            "the room holds four posts and the mark stopped at the first"
        );

        tree.fold
            .apply(&log_frame(10, room_cursor("reviewer", "itm_p4")));
        assert_eq!(of(&tree, "reviewer").and_then(|seat| seat.unread), None);
    }

    /// A mark is one seat's, under that seat's own name. Another seat's says
    /// nothing here, and neither does a payload of a shape this surface does
    /// not know or one naming a post the journal no longer holds — each leaves
    /// the fact out rather than guessing a number.
    #[test]
    fn a_mark_of_another_seat_or_another_shape_counts_nothing() {
        let mut tree = seated();
        tree.fold
            .apply(&log_frame(9, room_cursor("watcher", "itm_p1")));
        assert_eq!(of(&tree, "reviewer").and_then(|seat| seat.unread), None);
        assert_eq!(
            of(&tree, "watcher").and_then(|seat| seat.unread),
            Some(3),
            "it counts for the seat it was filed under"
        );

        tree.fold.apply(&log_frame(
            10,
            extended(
                "bingo.rooms",
                "cursor:reviewer",
                serde_json::json!({"seq": 1}),
            ),
        ));
        assert_eq!(of(&tree, "reviewer").and_then(|seat| seat.unread), None);

        tree.fold.apply(&log_frame(
            11,
            extended(
                "bingo.rooms",
                "cursor:reviewer",
                serde_json::json!({"post": "itm_gone"}),
            ),
        ));
        assert_eq!(of(&tree, "reviewer").and_then(|seat| seat.unread), None);
    }

    #[test]
    fn a_room_says_how_many_seats_it_has_and_how_many_of_them_owe() {
        let tree = seated();
        let room = tree
            .sessions()
            .find(|state| tree::name(state) == "#design")
            .expect("the room");
        assert_eq!(counts(&tree, room), Counts { seats: 2, owed: 1 });
    }

    /// The card is one debt per row; a member that owes twice is one debtor,
    /// and the oldest of its debts is the one its row says.
    #[test]
    fn a_member_that_owes_twice_is_one_debtor_and_says_the_older() {
        let mut tree = seated();
        tree.fold.apply(&frame(
            6,
            signalled(
                "bingo.rooms",
                "owed",
                owed_payload(&[("#design", "reviewer", 22), ("#design", "reviewer", 15)]),
            ),
        ));
        let room = tree
            .sessions()
            .find(|state| tree::name(state) == "#design")
            .expect("the room");
        assert_eq!(counts(&tree, room).owed, 1);
        assert_eq!(
            of(&tree, "reviewer").and_then(|seat| seat.owes),
            Some(asked_minutes_ago(22))
        );
    }

    /// Another room's debts are another room's: the table names the room in
    /// every row, and only the rows that name this one are read.
    #[test]
    fn a_debt_in_another_room_is_not_this_rooms() {
        let mut tree = seated();
        tree.fold.apply(&frame(
            6,
            signalled(
                "bingo.rooms",
                "owed",
                owed_payload(&[("#ops", "reviewer", 22)]),
            ),
        ));
        assert_eq!(of(&tree, "reviewer").and_then(|seat| seat.owes), None);
    }

    /// A card a process before the debts published: the clock time is all it
    /// has, so that is what the seat carries and what its row will say. This
    /// is a payload already in people's journals, not a shape this surface is
    /// free to stop reading.
    #[test]
    fn a_card_from_before_the_debts_says_the_clock_time_it_carries() {
        let mut tree = seated();
        tree.fold.apply(&frame(
            6,
            signalled(
                "bingo.rooms",
                "owed",
                owed_table_payload(&[("#design", "reviewer", "14:02")]),
            ),
        ));
        assert_eq!(
            of(&tree, "reviewer").and_then(|seat| seat.owes),
            Some(Owes::At("14:02".into()))
        );
    }

    /// A payload this surface does not recognise leaves the fact out. The
    /// journal is somebody else's, and a guess would be worse than a silence.
    #[test]
    fn a_payload_of_another_shape_says_nothing_rather_than_guessing() {
        let mut tree = seated();
        tree.fold.apply(&frame(
            6,
            signalled("bingo.rooms", "owed", serde_json::json!({"kind": "text"})),
        ));
        assert_eq!(of(&tree, "reviewer").and_then(|seat| seat.owes), None);

        tree.fold.apply(&log_frame(
            9,
            extended("bingo.rooms", "members", serde_json::json!(7)),
        ));
        assert_eq!(of(&tree, "reviewer"), None, "nobody is seated by a number");
    }
}
