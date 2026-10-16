//! Which part each item plays in its turn (ADR-0095 §6–§8): a step's row
//! while the turn runs, one folded line once it is over, the stop line where
//! the person stopped it. Worked out once per sync from the items and the
//! fold map, so the row that draws and the request that asks for a drawing
//! read the same answer.

use std::collections::BTreeMap;

use bingo_sdk::component::{LedgerProps, Props, Step, StepState, TurnEnd, TurnFoldProps};
use bingo_sdk::{
    DecisionKind, Item, ItemBody, ItemId, ItemStatus, SessionState, TurnStatus, Upshot, view::Tone,
};

use crate::fold::{self, Fold, Folds};
use crate::tree::Agents;
use crate::{paths, thoughts};

/// What one item draws as, within its turn.
#[derive(Clone, Debug, Default, PartialEq)]
pub enum Role {
    /// The item's own drawing: everything outside a turn's steps.
    #[default]
    Own,
    /// Folded away under its turn's line.
    Hidden,
    /// A step's row; `opened` when it stands under an opened turn's line,
    /// which is when its output hangs under it.
    Step { step: Step, opened: bool },
    /// The first step or thought of a finished turn holds its line, and under
    /// the line the part it plays when the turn is opened.
    Fold {
        fold: TurnFoldProps,
        then: Option<Box<Role>>,
    },
    /// A thought of an opened turn: its words, dim, under the steps.
    Thought,
    /// Where the person stopped the turn: how many of its steps had run.
    Stop { ran: usize, total: usize },
}

/// Every item's role, by id; an item not named plays its own.
#[derive(Debug, Default)]
pub struct Roles(BTreeMap<ItemId, Role>);

impl Roles {
    pub fn of(state: &SessionState, agents: &Agents<'_>, folds: &Folds) -> Self {
        let items: Vec<&Item> = state.transcript.items().iter().map(AsRef::as_ref).collect();
        let replaced = rowed(state);
        let mut roles = BTreeMap::new();
        for run in items.chunk_by(|a, b| a.turn.is_some() && a.turn == b.turn) {
            let Some(id) = run.first().and_then(|item| item.turn.as_ref()) else {
                continue;
            };
            let turn = Turn {
                items: run,
                state,
                agents,
                replaced: &replaced,
            };
            match state.turn.as_ref().is_some_and(|live| &live.id == id) {
                true => turn.live(&mut roles),
                false => turn.finished(folds, &mut roles),
            }
        }
        Self(roles)
    }

    pub fn of_item(&self, id: &ItemId) -> &Role {
        static OWN: Role = Role::Own;
        self.0.get(id).unwrap_or(&OWN)
    }

    /// Whether a step is running: its spinner turns on its own clock.
    pub fn spinning(&self) -> bool {
        self.0
            .values()
            .any(|role| matches!(role, Role::Step { step, .. } if step.state == StepState::Running))
    }

    /// The item holding the line `id` is folded or opened under, where a key
    /// or a click on `id` opens or shuts its turn; `None` for an item that is
    /// a row of its own.
    pub fn line_of(&self, state: &SessionState, id: &ItemId) -> Option<ItemId> {
        match self.of_item(id) {
            Role::Fold { .. } => Some(id.clone()),
            Role::Hidden | Role::Thought | Role::Step { opened: true, .. } => {
                self.holder(state, id)
            }
            _ => None,
        }
    }

    /// The item holding the line of the turn `id` is part of.
    fn holder(&self, state: &SessionState, id: &ItemId) -> Option<ItemId> {
        let turn = state.item(id)?.turn.as_ref()?;
        state
            .transcript
            .items()
            .iter()
            .map(AsRef::as_ref)
            .filter(|item| item.turn.as_ref() == Some(turn))
            .find(|item| matches!(self.of_item(&item.id), Role::Fold { .. }))
            .map(|item| item.id.clone())
    }
}

/// What an item asks its sites to draw for the part it plays: a step its
/// row, a folded line itself and, opened, the step under it. The ids are the
/// ones the drawing resolves with, so a request and its row meet.
pub fn requests(item: &Item, role: &Role) -> Vec<(String, Props)> {
    match role {
        Role::Step { step, .. } => vec![ledger(item, step)],
        Role::Fold { fold, then } => {
            let under = then.as_deref().map(|then| requests(item, then));
            std::iter::once(folded(item, fold))
                .chain(under.into_iter().flatten())
                .collect()
        }
        Role::Own | Role::Hidden | Role::Thought | Role::Stop { .. } => Vec::new(),
    }
}

/// A step's row as its site is asked for it.
pub fn ledger(item: &Item, step: &Step) -> (String, Props) {
    let props = Props::Ledger(LedgerProps {
        steps: vec![step.clone()],
    });
    (crate::component::items::id(item), props)
}

/// A finished turn's line as its site is asked for it, by the item holding it.
pub fn folded(item: &Item, fold: &TurnFoldProps) -> (String, Props) {
    (item.id.to_string(), Props::TurnFold(fold.clone()))
}

/// The items a producer drew a row of its own for: those keep their drawing.
fn rowed(state: &SessionState) -> Vec<ItemId> {
    bingo_sdk::slot::placed(state)
        .into_iter()
        .filter_map(|placed| match placed.slot {
            bingo_sdk::slot::Slot::Row { item, .. } => Some(item),
            _ => None,
        })
        .collect()
}

/// One turn's items, and what deciding their parts reads.
struct Turn<'a> {
    items: &'a [&'a Item],
    state: &'a SessionState,
    agents: &'a Agents<'a>,
    replaced: &'a [ItemId],
}

impl Turn<'_> {
    /// A running turn: each step its row, no thought a row of its own.
    fn live(&self, roles: &mut BTreeMap<ItemId, Role>) {
        let mut steps = Vec::new();
        for item in self.items {
            let role = match () {
                _ if self.is_step(item) => {
                    let step = step(item, self.state);
                    steps.push(step.clone());
                    Role::Step {
                        step,
                        opened: false,
                    }
                }
                _ if thoughts::is_thought(item) => Role::Hidden,
                _ => continue,
            };
            roles.insert(item.id.clone(), role);
        }
        self.stops(&steps, roles);
    }

    /// A turn that is over: one line on its first step or thought, the rest
    /// under it.
    fn finished(&self, folds: &Folds, roles: &mut BTreeMap<ItemId, Role>) {
        let steps: Vec<usize> = (0..self.items.len())
            .filter(|&at| self.is_step(self.items[at]))
            .collect();
        let thought = self.items.iter().any(|item| thoughts::is_thought(item));
        // The first of the working holds the line, so an opened turn reads
        // from its line down.
        let first = |item: &&&Item| self.is_step(item) || thoughts::is_thought(item);
        let Some(holder) = self.items.iter().position(|item| first(&item)) else {
            self.stops(&[], roles);
            return;
        };
        let opened = fold::fold_of(folds, self.items[holder]) == Fold::Open;
        let said = self.steps(&steps);
        self.stops(&said, roles);
        let fold = TurnFoldProps {
            steps: said,
            outcome: self.outcome(),
            elapsed: self.elapsed(),
            thought,
            is_expanded: opened,
        };
        let last_step = steps.last().copied();
        for (at, item) in self.items.iter().enumerate() {
            let role = match () {
                _ if at == holder => Role::Fold {
                    fold: fold.clone(),
                    then: opened.then(|| Box::new(self.opened(item))),
                },
                _ if self.is_step(item) || thoughts::is_thought(item) => match opened {
                    true => self.opened(item),
                    false => Role::Hidden,
                },
                _ if !opened && narrates(item, at, last_step) => Role::Hidden,
                // An answer from before receipts named their step: it is the
                // working's, and goes where the working goes.
                _ if !opened && matches!(item.body, ItemBody::PermissionReceipt { .. }) => {
                    Role::Hidden
                }
                _ => continue,
            };
            roles.insert(item.id.clone(), role);
        }
    }

    /// The stop line, wherever the turn says the person stopped it.
    fn stops(&self, steps: &[Step], roles: &mut BTreeMap<ItemId, Role>) {
        let ran = steps.iter().filter(|step| ran(step)).count();
        for item in self.items {
            if matches!(item.body, ItemBody::Interruption { .. }) {
                let total = steps.len();
                roles.insert(item.id.clone(), Role::Stop { ran, total });
            }
        }
    }

    /// What an item of an opened turn shows under the line.
    fn opened(&self, item: &Item) -> Role {
        match self.is_step(item) {
            true => Role::Step {
                step: step(item, self.state),
                opened: true,
            },
            false => Role::Thought,
        }
    }

    fn steps(&self, at: &[usize]) -> Vec<Step> {
        at.iter()
            .map(|&at| step(self.items[at], self.state))
            .collect()
    }

    /// A call the turn made that reads as a step. A call that draws its own
    /// session, a quiet one, and one a producer drew a row for are not.
    fn is_step(&self, item: &Item) -> bool {
        let ItemBody::ToolCall { quiet, output, .. } = &item.body else {
            return false;
        };
        let quietly = *quiet && !item.status.failed(output.as_ref());
        !quietly && !self.agents.contains_key(&item.id) && !self.replaced.contains(&item.id)
    }

    fn outcome(&self) -> TurnEnd {
        let stopped = self
            .items
            .iter()
            .any(|item| matches!(item.body, ItemBody::Interruption { .. }));
        let failed = self.state.last_turn.as_ref().is_some_and(|turn| {
            Some(&turn.id) == self.items[0].turn.as_ref()
                && matches!(turn.status, TurnStatus::Failed { .. })
        });
        match (stopped, failed) {
            (true, _) => TurnEnd::Stopped,
            (false, true) => TurnEnd::Failed,
            _ => TurnEnd::Done,
        }
    }

    /// From the turn's first item to the last moment any of them moved.
    fn elapsed(&self) -> u64 {
        let first = self.items.iter().map(|item| item.started_at).min();
        let last = self
            .items
            .iter()
            .map(|item| item.completed_at.unwrap_or(item.started_at))
            .max();
        match first.zip(last) {
            Some((first, last)) => {
                u64::try_from(last.duration_since(first).as_millis()).unwrap_or(0)
            }
            None => 0,
        }
    }
}

/// What the model said between its steps: the working, not the answer, which
/// is what it said after the last of them.
fn narrates(item: &Item, at: usize, last_step: Option<usize>) -> bool {
    matches!(item.body, ItemBody::Assistant { .. }) && last_step.is_some_and(|last| at < last)
}

/// Whether a step had run when its turn ended: it came back, and the person
/// had not said no to it.
fn ran(step: &Step) -> bool {
    let denied = matches!(
        &step.answer,
        Some(Upshot::Words {
            tone: Tone::Bad,
            ..
        })
    );
    matches!(step.state, StepState::Done | StepState::Failed) && !denied
}

/// One call, in the words its row shows.
pub fn step(item: &Item, state: &SessionState) -> Step {
    let ItemBody::ToolCall {
        call_id,
        name,
        input,
        output,
        ..
    } = &item.body
    else {
        return Step {
            verb: String::new(),
            target: String::new(),
            state: StepState::Done,
            upshot: None,
            answer: None,
        };
    };
    let external = item
        .external()
        .then(|| {
            bingo_sdk::ExternalCall::from_input(&bingo_sdk::ToolCall {
                call_id: call_id.clone(),
                name: name.clone(),
                input: input.clone(),
            })
        })
        .flatten();
    let (verb, target) = match &external {
        Some(call) => (call.kind.as_str().to_owned(), call.title.clone()),
        None => (verb(name), crate::transcript::summarize(input)),
    };
    let target = target.lines().next().unwrap_or_default();
    Step {
        verb,
        target: paths::shorten_in(target, &state.summary.cwd, paths::home()),
        state: step_state(item, state),
        upshot: output.as_ref().and_then(bingo_sdk::ToolOutput::outcome),
        answer: answer(item, state),
    }
}

/// A tool's name as a row says it: `Read` is `read`, `WebSearch` is
/// `web search`.
fn verb(name: &str) -> String {
    let mut out = String::with_capacity(name.len() + 2);
    let mut lower = false;
    for c in name.chars() {
        if c.is_uppercase() && lower {
            out.push(' ');
        }
        lower = c.is_lowercase();
        out.extend(c.to_lowercase());
    }
    out
}

fn step_state(item: &Item, state: &SessionState) -> StepState {
    let ItemBody::ToolCall { output, .. } = &item.body else {
        return StepState::Done;
    };
    let asked = || {
        state
            .interactions
            .iter()
            .any(|open| open.item.as_ref() == Some(&item.id))
    };
    match item.status {
        ItemStatus::Pending | ItemStatus::Running if asked() => StepState::Asked,
        ItemStatus::Pending | ItemStatus::Running => StepState::Running,
        ItemStatus::Interrupted => StepState::Stopped,
        status if status.failed(output.as_ref()) => StepState::Failed,
        _ => StepState::Done,
    }
}

/// What the person answered when the step asked, in the receipt's words.
fn answer(item: &Item, state: &SessionState) -> Option<Upshot> {
    let receipts: Vec<&Item> = crate::ask::receipts::linked(state, &item.id).collect();
    let words: Vec<String> = receipts
        .iter()
        .filter_map(|receipt| crate::ask::receipts::label(&receipt.body))
        .collect();
    if words.is_empty() {
        return None;
    }
    let refused = receipts.iter().any(|receipt| {
        matches!(
            receipt.body,
            ItemBody::PermissionReceipt {
                decision: DecisionKind::Deny,
                ..
            }
        )
    });
    Some(Upshot::Words {
        text: words.join(" \u{b7} "),
        tone: match refused {
            true => Tone::Bad,
            false => Tone::Neutral,
        },
    })
}

#[cfg(test)]
#[path = "turns_tests.rs"]
mod tests;
