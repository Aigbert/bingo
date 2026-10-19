//! Borrowed choices for the stream frame.
use crate::fold::Folds;
use bingo_sdk::{Item, ItemId};

/// Borrowed expansion choices for one transcript frame.
#[derive(Clone, Copy)]
pub struct Opened<'a> {
    pub folds: &'a Folds,
    pub detail: Option<&'a bingo_sdk::TurnId>,
    pub wells: Option<&'a std::collections::BTreeMap<bingo_sdk::TurnId, ItemId>>,
    pub browse_hit: Option<&'a ItemId>,
}

impl<'a> Opened<'a> {
    /// Whether an item belongs to the turn detail holds open.
    pub fn in_detail(&self, item: &Item) -> bool {
        self.detail
            .is_some_and(|turn| item.turn.as_ref() == Some(turn))
    }
    pub fn well(&self, item: &Item) -> bool {
        self.browse_hit == Some(&item.id)
            || item
                .turn
                .as_ref()
                .is_some_and(|turn| self.wells.and_then(|wells| wells.get(turn)) == Some(&item.id))
    }
    pub fn new(folds: &'a Folds) -> Self {
        Self {
            folds,
            detail: None,
            wells: None,
            browse_hit: None,
        }
    }
}
