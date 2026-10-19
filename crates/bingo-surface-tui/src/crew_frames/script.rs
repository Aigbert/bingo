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
    /// Turn time from which every beat lands later, and by how much: a turn
    /// waits while it asks.
    pause: (i64, i64),
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
            pause: (i64::MAX, 0),
            beats: Vec::new(),
        };
        turn.at(0, started(id));
        let prompt = turn.stamped(prompt, 0, Some(0));
        turn.at(0, Event::ItemCompleted { item: prompt });
        turn
    }

    /// The same turn, picked up where it asked at `asked` and was answered
    /// `ms` later; its first `steps` items are already journaled.
    pub fn resumed(
        session: &SessionId,
        id: &str,
        t0: i64,
        steps: usize,
        pause: (i64, i64),
    ) -> Self {
        Self {
            session: session.clone(),
            id: id.into(),
            t0,
            open: None,
            n: steps,
            pause,
            beats: Vec::new(),
        }
    }

    /// When `at` happens, from the scene's setup.
    fn moment(&self, at: i64) -> i64 {
        let (from, ms) = self.pause;
        self.t0 + at + if at > from { ms } else { 0 }
    }

    fn at(&mut self, at: i64, event: Event) {
        self.beats.push(Beat {
            t: self.moment(at),
            session: self.session.clone(),
            event,
        });
    }

    fn next_id(&mut self) -> String {
        self.n += 1;
        format!("{}-{}", self.id, self.n)
    }

    fn stamp(&self, at: i64) -> Timestamp {
        Timestamp::from_millisecond(SETUP + self.moment(at)).unwrap()
    }

    fn stamped(&self, mut item: Item, at: i64, end: Option<i64>) -> Item {
        item.turn = Some(TurnId::from_raw(&self.id));
        item.started_at = self.stamp(at);
        if let ItemBody::ToolCall { began_at, .. } = &mut item.body {
            *began_at = Some(item.started_at);
        }
        item.completed_at = end.map(|end| self.stamp(end));
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
        let call = self.reopen(at, name, arg);
        self.at(at, Event::ItemStarted { item: call.clone() });
        call.id.to_string()
    }

    /// The call `B.step(at, tool, arg)` opened, still open, without
    /// journaling it again.
    pub fn reopen(&mut self, at: i64, name: &str, arg: &str) -> Item {
        let id = self.next_id();
        let input = serde_json::json!({ "target": arg });
        let mut call = tool(&id, name, input, None, ItemStatus::Running);
        if let ItemBody::ToolCall { call_id, .. } = &mut call.body {
            *call_id = format!("call_{id}");
        }
        let call = self.stamped(call, at, None);
        self.open = Some(call.clone());
        call
    }

    /// The open call asked at the pause and began once answered, as the
    /// kernel records a run that waited on you.
    pub fn waited(&mut self) {
        let (from, ms) = self.pause;
        let at = |t: i64| Timestamp::from_millisecond(SETUP + self.t0 + t).unwrap();
        let (asked, began) = (at(from), at(from + ms));
        if let Some(Item {
            body: ItemBody::ToolCall {
                asked_at, began_at, ..
            },
            ..
        }) = &mut self.open
        {
            *asked_at = Some(asked);
            *began_at = Some(began);
        }
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
        let started = if started > self.pause.0 {
            started - self.pause.1
        } else {
            started
        };
        let began = match &call.body {
            ItemBody::ToolCall { began_at, .. } => *began_at,
            _ => None,
        };
        let mut call = self.stamped(call, started, Some(at));
        if let ItemBody::ToolCall { began_at, .. } = &mut call.body {
            *began_at = began.or(*began_at);
        }
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
            t: self.moment(at),
            session: session.clone(),
            event,
        });
    }
}
