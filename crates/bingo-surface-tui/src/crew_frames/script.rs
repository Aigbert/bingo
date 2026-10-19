//! A prototype script (`B.think`, `B.step`, `B.ok`, `B.reply` in p4-model.js)
//! as the events a turn journals, each at its moment of the scene.
use super::world::SETUP;
use crate::test_support::*;
use bingo_sdk::{Event, Item, ItemBody, ItemStatus, SessionId, ToolOutput, TurnId, TurnStatus};
use jiff::Timestamp;

/// The prototype's answer rate: characters a second.
const RATE_R: i64 = 150;

/// One event of a session, `t` milliseconds after the scene's setup.
pub struct Beat {
    pub t: i64,
    pub session: SessionId,
    pub event: Event,
}

/// One turn of one session, written beat by beat from its start `t0`.
pub struct Turn {
    session: SessionId,
    id: String,
    t0: i64,
    open: Option<Item>,
    n: usize,
    pub beats: Vec<Beat>,
}

impl Turn {
    /// A turn that opens on `prompt`, the item `startTurn()` pushes.
    pub fn start(session: &SessionId, id: &str, t0: i64, prompt: Item) -> Self {
        let mut turn = Self {
            session: session.clone(),
            id: id.into(),
            t0,
            open: None,
            n: 0,
            beats: Vec::new(),
        };
        turn.at(0, started(id));
        let prompt = turn.stamped(prompt, 0, Some(0));
        turn.at(0, Event::ItemCompleted { item: prompt });
        turn
    }

    fn at(&mut self, at: i64, event: Event) {
        self.beats.push(Beat {
            t: self.t0 + at,
            session: self.session.clone(),
            event,
        });
    }

    fn next_id(&mut self) -> String {
        self.n += 1;
        format!("{}-{}", self.id, self.n)
    }

    fn stamped(&self, mut item: Item, at: i64, end: Option<i64>) -> Item {
        item.turn = Some(TurnId::from_raw(&self.id));
        let moment = |at: i64| Timestamp::from_millisecond(SETUP + self.t0 + at).unwrap();
        item.started_at = moment(at);
        if let ItemBody::ToolCall { began_at, .. } = &mut item.body {
            *began_at = Some(item.started_at);
        }
        item.completed_at = end.map(moment);
        item
    }

    /// `B.think(at, ms, text)`.
    pub fn think(&mut self, at: i64, ms: i64, text: &str) {
        let body = ItemBody::Reasoning {
            text: text.into(),
            provider_metadata: Default::default(),
        };
        let id = self.next_id();
        let running = self.stamped(item(&id, ItemStatus::Running, body.clone()), at, None);
        self.at(at, Event::ItemStarted { item: running });
        let done = self.stamped(item(&id, ItemStatus::Completed, body), at, Some(at + ms));
        self.at(at + ms, Event::ItemCompleted { item: done });
    }

    /// `B.step(at, tool, arg)`: the call starts and stays open. Returns its id.
    pub fn step(&mut self, at: i64, name: &str, arg: &str) -> String {
        let id = self.next_id();
        let input = serde_json::json!({ "target": arg });
        let mut call = tool(&id, name, input, None, ItemStatus::Running);
        if let ItemBody::ToolCall { call_id, .. } = &mut call.body {
            *call_id = format!("call_{id}").into();
        }
        let call = self.stamped(call, at, None);
        self.at(at, Event::ItemStarted { item: call.clone() });
        self.open = Some(call);
        id
    }

    /// `B.ok(at, res)`: the open call lands with `res` as its upshot.
    pub fn ok(&mut self, at: i64, res: &str) {
        let Some(mut call) = self.open.take() else {
            panic!("ok without an open step");
        };
        let ItemBody::ToolCall { output, .. } = &mut call.body else {
            panic!("only a call lands");
        };
        let landed = ToolOutput::text(res).with_upshot(bingo_sdk::Upshot::plain(res));
        *output = Some(Box::new(landed));
        call.status = ItemStatus::Completed;
        let started = call.started_at.as_millisecond() - SETUP - self.t0;
        let call = self.stamped(call, started, Some(at));
        self.at(at, Event::ItemCompleted { item: call });
    }

    /// `B.reply(at, …)`: the answer streams at `RATE_R`, then the turn ends.
    /// Returns the moment it ends, from the turn's start.
    pub fn reply(&mut self, at: i64, text: &str) -> i64 {
        let end = at + text.chars().count() as i64 * 1000 / RATE_R;
        let id = self.next_id();
        let running = self.stamped(assistant(&id, text, ItemStatus::Running), at, None);
        self.at(at, Event::ItemStarted { item: running });
        let done = self.stamped(assistant(&id, text, ItemStatus::Completed), at, Some(end));
        self.at(end, Event::ItemCompleted { item: done });
        self.at(end, completed(&self.id.clone(), TurnStatus::Completed));
        end
    }

    /// Something the turn does that is not one of its items, at `at`.
    pub fn call(&mut self, at: i64, session: &SessionId, event: Event) {
        self.beats.push(Beat {
            t: self.t0 + at,
            session: session.clone(),
            event,
        });
    }
}
