//! Inputs that arrived while the session was busy, drained one unit at a time
//! (ADR-0008 §2): a run of prose opens one turn, a command is one unit, and a
//! barrier absorbs prose only up to the first line that will not steer — a
//! command, or a line that asked to wait for the turn to end (M68).

use std::collections::VecDeque;

use bingo_sdk::*;

use super::commands;

/// Characters of a queued input shown in `QueueChanged`.
const PREVIEW_CHARS: usize = 80;

#[derive(Default)]
pub(super) struct Queue {
    entries: VecDeque<(IntentId, Pending)>,
    revision: u64,
}

/// Delivery timing is already on `Input`; this preserves how it arrived.
pub(super) enum Pending {
    Submitted(Input),
    Delivered(Input),
}

impl From<Input> for Pending {
    fn from(input: Input) -> Self {
        Self::Submitted(input)
    }
}

impl Pending {
    fn input(&self) -> &Input {
        match self {
            Self::Submitted(input) | Self::Delivered(input) => input,
        }
    }

    fn into_input(self) -> Input {
        match self {
            Self::Submitted(input) | Self::Delivered(input) => input,
        }
    }

    fn is_command(&self) -> bool {
        matches!(self, Self::Submitted(input) if commands::is_command(input))
    }

    fn steerable(&self) -> bool {
        !self.is_command() && !self.input().is_held()
    }
}

/// What the head of the queue asks the idle session to do next.
pub(super) enum Unit {
    Prose(Vec<(IntentId, Input)>),
    Command(IntentId, Input),
}

impl Queue {
    /// A wake or command already waiting keeps its place ahead of new input.
    pub(super) fn ready(&self) -> bool {
        self.entries
            .iter()
            .any(|(_, input)| !input.input().is_held() || input.is_command())
    }

    /// Append; returns the 1-based position.
    pub(super) fn push(&mut self, intent: IntentId, input: impl Into<Pending>) -> u32 {
        self.entries.push_back((intent, input.into()));
        self.entries.len() as u32
    }

    /// Everything, for a session that is closing.
    pub(super) fn drain_all(&mut self) -> Vec<(IntentId, Input)> {
        self.entries.drain(..).map(unpack).collect()
    }

    /// The prose at the head, up to the first command: what a turn opening
    /// now takes with it. A held line is prose here — it asked to wait for a
    /// turn, and this is the turn it waited for.
    pub(super) fn take_prose(&mut self) -> Vec<(IntentId, Input)> {
        self.take_run(|input| !input.is_command())
    }

    /// The prose a running turn may be steered with: up to the first command
    /// and the first line that asked to wait (M68).
    pub(super) fn take_steers(&mut self) -> Vec<(IntentId, Input)> {
        self.take_run(Pending::steerable)
    }

    /// The head of the queue for as long as `keep` holds.
    fn take_run(&mut self, keep: impl Fn(&Pending) -> bool) -> Vec<(IntentId, Input)> {
        let n = self
            .entries
            .iter()
            .take_while(|(_, input)| keep(input))
            .count();
        self.entries.drain(..n).map(unpack).collect()
    }

    /// What `intent` is queued as, while it still is: what a withdraw reads
    /// before it decides whether the line is the caller's to take.
    pub(super) fn queued(&self, intent: &IntentId) -> Option<&Input> {
        self.entries
            .iter()
            .find(|(queued, _)| queued == intent)
            .map(|(_, input)| input.input())
    }

    /// Take `intent` out of the queue; `None` when it is not in it.
    pub(super) fn take(&mut self, intent: &IntentId) -> Option<Input> {
        let at = self.entries.iter().position(|(id, _)| id == intent)?;
        self.entries.remove(at).map(|(_, input)| input.into_input())
    }

    /// The next unit, when there is one.
    pub(super) fn take_unit(&mut self) -> Option<Unit> {
        let (_, head) = self.entries.front()?;
        if head.is_command() {
            return self
                .entries
                .pop_front()
                .map(|(i, input)| Unit::Command(i, input.into_input()));
        }
        Some(Unit::Prose(self.take_prose()))
    }

    /// The wire view after a change; every call is a new revision.
    pub(super) fn changed(&mut self) -> Event {
        self.revision += 1;
        Event::QueueChanged {
            revision: self.revision,
            entries: self
                .entries
                .iter()
                .enumerate()
                .map(|(i, (intent, input))| entry(intent, input, i as u32 + 1))
                .collect(),
        }
    }
}

fn entry(intent: &IntentId, input: &Pending, position: u32) -> QueueEntry {
    QueueEntry {
        intent: intent.clone(),
        position,
        preview: preview(input.input()),
        steerable: input.steerable(),
        origin: origin_of(input.input()),
    }
}

fn unpack((intent, input): (IntentId, Pending)) -> (IntentId, Input) {
    (intent, input.into_input())
}

pub(super) fn origin_of(input: &Input) -> Origin {
    match input {
        Input::Text { origin, .. } => origin.clone(),
        Input::Action { .. } => Origin::default(),
    }
}

/// The words shown for a queued ask, cut to length; an ask that carries a
/// picture and no words previews as `(an image)`.
fn preview(input: &Input) -> String {
    let text = match input {
        Input::Text { text, images, .. } if text.is_empty() && !images.is_empty() => {
            return "(an image)".to_string();
        }
        Input::Text { text, .. } => text.as_str(),
        Input::Action { action, .. } => action.name.as_str(),
    };
    bingo_sdk::command::recorded_line(text)
        .chars()
        .take(PREVIEW_CHARS)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn provider_command_arguments_never_enter_queue_events() {
        let mut queue = Queue::default();
        let (id, input) = text("/provider add x openai https://u:SECRET@host EXTRA_SECRET");
        queue.push(id, input);
        assert!(
            !serde_json::to_string(&queue.changed())
                .unwrap()
                .contains("SECRET")
        );
    }

    #[test]
    fn delivered_command_spelling_stays_prose_with_either_timing() {
        for spelling in ["!touch marker", "/plugins trust probe full"] {
            for delivery in [Delivery::Wake, Delivery::Hold] {
                let input = Input::Text {
                    text: spelling.into(),
                    images: Vec::new(),
                    origin: Origin::surface("rpc"),
                    delivery,
                };
                let mut queue = Queue::default();
                queue.push(IntentId::mint(), Pending::Delivered(input.clone()));
                assert_eq!(queue.ready(), delivery == Delivery::Wake);
                let Event::QueueChanged { entries, .. } = queue.changed() else {
                    panic!("queue event");
                };
                assert_eq!(entries[0].steerable, delivery == Delivery::Wake);
                if delivery == Delivery::Hold {
                    assert!(queue.take_steers().is_empty());
                }
                assert!(
                    matches!(queue.take_unit(), Some(Unit::Prose(lines)) if lines[0].1 == input)
                );
            }
        }
    }

    fn text(t: &str) -> (IntentId, Input) {
        (IntentId::mint(), Input::text(t, Origin::surface("rpc")))
    }

    /// A line that asked to wait for the turn to end (M68).
    fn held(t: &str) -> (IntentId, Input) {
        (
            IntentId::mint(),
            Input::Text {
                text: t.into(),
                images: Vec::new(),
                origin: Origin::surface("rpc"),
                delivery: Delivery::Hold,
            },
        )
    }

    fn push(q: &mut Queue, entry: (IntentId, Input)) -> IntentId {
        q.push(entry.0.clone(), entry.1);
        entry.0
    }

    #[test]
    fn prose_is_one_unit_and_a_command_is_another() {
        let mut q = Queue::default();
        q.push(text("a").0, text("a").1);
        q.push(text("b").0, text("b").1);
        q.push(text("/compact").0, text("/compact").1);
        q.push(text("c").0, text("c").1);
        assert!(matches!(q.take_unit(), Some(Unit::Prose(p)) if p.len() == 2));
        assert!(matches!(q.take_unit(), Some(Unit::Command(..))));
        assert!(matches!(q.take_unit(), Some(Unit::Prose(p)) if p.len() == 1));
        assert!(q.take_unit().is_none());
    }

    #[test]
    fn a_barrier_absorbs_prose_only_up_to_the_first_command() {
        let mut q = Queue::default();
        q.push(text("a").0, text("a").1);
        q.push(text("!ls").0, text("!ls").1);
        q.push(text("b").0, text("b").1);
        assert_eq!(q.take_prose().len(), 1);
        assert!(q.take_prose().is_empty(), "the command blocks the rest");
        assert!(matches!(q.take_unit(), Some(Unit::Command(..))));
    }

    #[test]
    fn the_wire_view_marks_commands_as_not_steerable() {
        let mut q = Queue::default();
        q.push(text("a").0, text("a").1);
        q.push(text("/x").0, text("/x").1);
        let Event::QueueChanged { revision, entries } = q.changed() else {
            panic!("a queue change");
        };
        assert_eq!(revision, 1);
        assert_eq!(entries[0].position, 1);
        assert!(entries[0].steerable);
        assert!(!entries[1].steerable);
    }

    #[test]
    fn an_image_only_ask_previews_as_an_image() {
        let image = Image::from_bytes("image/png", b"abc").expect("a small image");
        let input = Input::Text {
            text: String::new(),
            images: vec![image],
            origin: Origin::surface("rpc"),
            delivery: Delivery::Wake,
        };
        let mut q = Queue::default();
        q.push(IntentId::mint(), input);
        let Event::QueueChanged { entries, .. } = q.changed() else {
            panic!("a queue change");
        };
        assert_eq!(entries[0].preview, "(an image)");
    }

    /// The barrier is what `Hold` is about: a running turn is steered with
    /// the lines that steer and with nothing after the first that does not.
    #[test]
    fn a_barrier_stops_at_a_held_line_as_it_stops_at_a_command() {
        let mut q = Queue::default();
        push(&mut q, text("a"));
        push(&mut q, held("b"));
        push(&mut q, text("c"));
        assert_eq!(q.take_steers().len(), 1, "only the line that steers");
        assert!(q.take_steers().is_empty(), "the held line blocks the rest");
    }

    /// And the turn it waited for takes it: a held line is prose to whatever
    /// opens next, so two of them are one turn's inputs.
    #[test]
    fn the_turn_that_opens_next_takes_the_held_lines() {
        let mut q = Queue::default();
        push(&mut q, held("a"));
        push(&mut q, held("b"));
        push(&mut q, text("/compact"));
        assert!(matches!(q.take_unit(), Some(Unit::Prose(p)) if p.len() == 2));
        assert!(matches!(q.take_unit(), Some(Unit::Command(..))));
    }

    #[test]
    fn the_wire_view_marks_a_held_line_as_not_steerable() {
        let mut q = Queue::default();
        push(&mut q, text("a"));
        push(&mut q, held("b"));
        let Event::QueueChanged { entries, .. } = q.changed() else {
            panic!("a queue change");
        };
        assert!(entries[0].steerable);
        assert!(!entries[1].steerable, "it asked to wait");
    }

    /// What a withdraw reads and then takes; an intent that is not queued is
    /// nothing to either of them.
    #[test]
    fn a_queued_line_is_found_by_its_intent_and_taken_out_once() {
        let mut q = Queue::default();
        let first = push(&mut q, text("a"));
        let second = push(&mut q, text("b"));
        assert!(q.queued(&first).is_some());
        assert!(q.queued(&IntentId::mint()).is_none());
        let taken = q.take(&first).expect("the first line");
        assert_eq!(preview(&taken), "a");
        assert!(q.take(&first).is_none(), "once");
        let Event::QueueChanged { entries, .. } = q.changed() else {
            panic!("a queue change");
        };
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].intent, second);
        assert_eq!(entries[0].position, 1, "what is left is renumbered");
    }
}
