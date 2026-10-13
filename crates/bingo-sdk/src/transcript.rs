//! The client and model orders over shared transcript items.

use crate::{Applied, Event, Item, ItemId};
use std::sync::Arc;

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Transcript;

impl Transcript {
    pub fn new() -> Self {
        Self
    }
    pub fn apply(&mut self, _event: &Event) -> Applied {
        Applied::Nothing
    }
    pub fn items(&self) -> &[Arc<Item>] {
        &[]
    }
    pub fn get(&self, _id: &ItemId) -> Option<&Item> {
        None
    }
    pub fn generation(&self) -> u64 {
        0
    }
    pub fn model_items(&self) -> impl Iterator<Item = &Item> {
        self.items().iter().map(AsRef::as_ref)
    }
}

#[cfg(test)]
mod tests;
