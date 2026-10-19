//! The crew world of p4d-crew.js, folded through the session boundary: home
//! with its past, three teammates and the #design room, each a session.
use crate::test_support::ember::{CLOCK_ORIGIN_MS, story};
use crate::test_support::*;
use crate::tree::Tree;
use bingo_sdk::{Effort, Event, Frame, Item, ParentLink, SessionId, SessionSummary};
use jiff::Timestamp;
use serde_json::json;

pub const ROOM: &str = "ses_design";

/// `ROOM_SAYS.reviewer` in p4d-crew.js.
pub const REVIEWER_SAYS: &str = "注释写的是「只重置 steer 槽」，和代码一致。建议补一句为什么排队的消息属于下一轮，不然下一个人还会把 `clear()` 加回去。";

/// The moment the scene's setup runs, on the prototype's private clock.
pub const SETUP: i64 = CLOCK_ORIGIN_MS;

pub struct World {
    pub tree: Tree,
}

pub fn id(name: &str) -> SessionId {
    SessionId::from_raw(&format!("ses_{name}"))
}

impl World {
    /// `past()`: the two finished turns of the done scene, at home.
    pub fn home() -> Self {
        Self::past(None)
    }

    /// `S.mode = 1; past()`: home in the mode that lets edits run.
    pub fn accepting_edits() -> Self {
        Self::past(Some("acceptEdits"))
    }

    fn past(mode: Option<&str>) -> Self {
        let (mut state, _) = story::scene("done");
        let mut summary = state.summary.clone();
        summary.model = Some("opus-5.5".into());
        summary.effort = Some(Effort::High);
        state.apply(&frame(state.seq.0 + 1, Event::SessionUpdated { summary }));
        if let Some(mode) = mode {
            state.apply(&frame(state.seq.0 + 1, permission_view(mode)));
        }
        state.config.kernel["render"] = json!({"Ledger": [{}]});
        Self {
            tree: Tree::new(state),
        }
    }

    pub fn emit(&mut self, session: &SessionId, at: i64, event: Event) {
        let seq = self.tree.state(session).map_or(0, |state| state.seq.0).max(
            if self.tree.is_root(session) {
                self.tree.root().seq.0
            } else {
                0
            },
        );
        let frame = Frame {
            session: session.clone(),
            ts: Timestamp::from_millisecond(at).unwrap(),
            ..frame(seq + 1, event)
        };
        self.tree.fold.apply(&frame);
    }

    /// A teammate from the roster, hired at `at`: the order its ink follows.
    pub fn mate(&mut self, name: &str, at: i64) {
        let event = hired(self.tree.root_id(), name, at, None);
        self.emit(&id(name), at, event);
    }

    pub fn post(&mut self, room: &SessionId, item: &str, by: &str, text: &str, at: i64) {
        self.emit(room, at, post_event(item, by, text, at));
    }
}

/// A roster teammate's session, opened at `at` by the call `item` of home,
/// or by none.
pub fn hired(home: &SessionId, name: &str, at: i64, item: Option<&str>) -> Event {
    let row = crate::test_support::ember::world::rows("roster")
        .into_iter()
        .find(|row| row["name"] == name)
        .unwrap();
    let summary = SessionSummary {
        id: id(name),
        title: Some(name.into()),
        parent: Some(ParentLink {
            session: home.clone(),
            item: item.map(bingo_sdk::ItemId::from_raw),
        }),
        created_at: Timestamp::from_millisecond(at).unwrap(),
        updated_at: Timestamp::from_millisecond(at).unwrap(),
        ..summary()
    };
    let mut summary = summary;
    summary.model = row["model"]["id"].as_str().map(str::to_string);
    summary.effort = row["model"]["eff"]
        .as_str()
        .and_then(|eff| serde_json::from_value(json!(eff)).ok());
    Event::SessionUpdated { summary }
}

/// The role `openAgent()` writes under a teammate's name, as the agents
/// plugin publishes it.
pub fn role(name: &str) -> Event {
    let row = crate::test_support::ember::world::rows("roster")
        .into_iter()
        .find(|row| row["name"] == name)
        .unwrap();
    extended(
        bingo_collab::role::PLUGIN,
        bingo_collab::role::KIND,
        bingo_collab::role::payload(row["role"].as_str().unwrap()),
    )
}

impl World {
    /// `designRoom()`: opened nine minutes before the scene, with its four posts.
    pub fn design_room(&mut self) {
        let at = SETUP - 9 * 60_000;
        let room = SessionId::from_raw(ROOM);
        let summary = SessionSummary {
            id: room.clone(),
            key: Some("rooms/design".into()),
            ..log_summary("#design")
        };
        let summary = SessionSummary {
            created_at: Timestamp::from_millisecond(at).unwrap(),
            updated_at: Timestamp::from_millisecond(at).unwrap(),
            ..summary
        };
        self.emit(&room, at, Event::SessionUpdated { summary });
        let opened = bingo_collab::room::Opened {
            purpose: Some("decide how reset() treats queued lines".into()),
            by: "parent".into(),
        };
        self.emit(
            &room,
            at,
            extended("bingo.rooms", "opened", opened.payload()),
        );
        self.emit(
            &room,
            at,
            extended(
                "bingo.rooms",
                "members",
                roster_payload(&["reviewer", "scout", "ada"], &[]),
            ),
        );
        for (n, (by, text, after)) in [
            (
                "parent",
                "`reset()` 要不要清空排队的消息？@ada @scout 说说看",
                1_000,
            ),
            (
                "ada",
                "我倾向于 `reset()` 完全不碰队列：排队的消息是你说过的话，属于下一轮。",
                40_000,
            ),
            (
                "scout",
                "`drain()` 有 4 处调用，都在一轮开始时，在 `reset()` 之后。",
                70_000,
            ),
            (
                "scout",
                "所以 `reset()` 不清队列，`drain()` 也能拿到它们。",
                75_000,
            ),
        ]
        .into_iter()
        .enumerate()
        {
            self.post(&room, &format!("room-{n}"), by, text, at + after);
        }
    }
}

/// A post signed by `by`, outside any turn, at `at`.
pub fn post_event(item: &str, by: &str, text: &str, at: i64) -> Event {
    let mut item = Item {
        turn: None,
        ..post(item, by, text)
    };
    item.started_at = Timestamp::from_millisecond(at).unwrap();
    item.completed_at = Some(item.started_at);
    Event::ItemCompleted { item }
}
