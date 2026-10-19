//! One frame's indexes into the transcript, shared by sync, refresh and requests.
use bingo_sdk::{Item, ItemId, SessionState, TurnId};
use std::{
    collections::{BTreeMap, BTreeSet},
    rc::Rc,
};

pub struct History<'a> {
    pub turns: BTreeMap<Option<TurnId>, Vec<&'a Item>>,
    pub followed: BTreeSet<&'a ItemId>,
    pub newest: Option<&'a TurnId>,
    pub positions: BTreeMap<&'a ItemId, usize>,
    pub answers: crate::ask::receipts::Answers<'a>,
}

impl<'a> History<'a> {
    pub fn of(state: &'a SessionState) -> Rc<Self> {
        let mut history = Self {
            turns: BTreeMap::new(),
            newest: None,
            followed: BTreeSet::new(),
            positions: BTreeMap::new(),
            answers: crate::ask::receipts::Answers::of(state),
        };
        for (at, item) in state.transcript.items().iter().enumerate() {
            history
                .turns
                .entry(item.turn.clone())
                .or_default()
                .push(item);
            history.positions.insert(&item.id, at);
            if item.turn.is_some() {
                history.newest = item.turn.as_ref();
            }
        }
        history.followed = history
            .turns
            .values()
            .flat_map(|items| crate::turns::followed(items))
            .collect();
        Rc::new(history)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::*;
    use bingo_sdk::{Event, ItemStatus};

    #[test]
    fn one_history_indexes_turn_order_positions_and_the_newest_turn() {
        let state = session(vec![
            frame(
                1,
                Event::ItemCompleted {
                    item: user("first", "first turn"),
                },
            ),
            frame(
                2,
                Event::ItemCompleted {
                    item: assistant("answer", "done", ItemStatus::Completed),
                },
            ),
            frame(
                3,
                Event::ItemCompleted {
                    item: user("second", "second turn"),
                },
            ),
        ]);
        let history = History::of(&state);
        let items = state.transcript.items();
        assert_eq!(history.turns.len(), 2);
        assert_eq!(
            history.turns[&items[0].turn]
                .iter()
                .map(|item| item.id.as_str())
                .collect::<Vec<_>>(),
            ["first", "answer"]
        );
        assert_eq!(history.positions[&items[2].id], 2);
        assert_eq!(history.newest, items[2].turn.as_ref());
    }
}
