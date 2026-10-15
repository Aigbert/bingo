//! The client and model orders over shared transcript items.

pub(crate) mod checkpoint;
mod find;
mod turns;
pub use find::{Match, find, text};
pub use turns::{HEADLINE_CHARS, Turn, from, turns};

use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use schemars::JsonSchema;
use serde::{Deserialize, Deserializer, Serialize, Serializer};

use crate::{Applied, DeltaKind, Event, Item, ItemBody, ItemId};

#[derive(Clone, Debug, Default, PartialEq, JsonSchema)]
pub struct Transcript {
    #[schemars(rename = "historyGeneration", default)]
    generation: u64,
    /// Items in transcript order, including every non-terminal one.
    #[schemars(with = "Vec<Item>")]
    items: Vec<Arc<Item>>,
    #[schemars(skip)]
    positions: HashMap<ItemId, usize>,
    #[schemars(skip)]
    model_order: Vec<ItemId>,
    #[schemars(skip)]
    model_ids: HashSet<ItemId>,
    #[schemars(skip)]
    hidden: usize,
    #[schemars(skip)]
    cut: Option<ItemId>,
    #[schemars(skip)]
    has_more: bool,
}

impl Transcript {
    /// Release only items that neither view needs; retained prefix items
    /// remain addressable by the model but are not client history.
    pub fn drop_before_cut(&mut self) {
        let cut = self
            .cut
            .as_ref()
            .and_then(|id| self.positions.get(id))
            .copied()
            .unwrap_or(self.hidden)
            .max(self.hidden);
        self.has_more |= cut > self.hidden;
        let mut at = 0;
        let mut hidden = 0;
        self.items.retain(|item| {
            let prefix = at < cut;
            at += 1;
            let keep = !prefix || self.model_ids.contains(&item.id);
            hidden += usize::from(prefix && keep);
            keep
        });
        self.hidden = hidden;
        self.positions = positions(&self.items);
    }

    pub fn history(&self) -> crate::OpenHistory {
        crate::OpenHistory {
            before: self.items().first().map(|item| item.id.clone()),
            has_more: self.has_more,
            generation: self.generation,
        }
    }

    pub fn new() -> Self {
        Self::default()
    }

    pub fn items(&self) -> &[Arc<Item>] {
        &self.items[self.hidden..]
    }

    pub fn get(&self, id: &ItemId) -> Option<&Item> {
        self.positions.get(id).map(|&at| self.items[at].as_ref())
    }

    /// Open items owned by a turn, in transcript order.
    pub fn open_items<'a>(&'a self, turn: &'a crate::TurnId) -> impl Iterator<Item = &'a Item> {
        self.items
            .iter()
            .map(AsRef::as_ref)
            .filter(move |item| item.turn.as_ref() == Some(turn) && !item.status.is_terminal())
    }

    pub fn generation(&self) -> u64 {
        self.generation
    }

    pub fn model_items(&self) -> impl Iterator<Item = &Item> {
        self.model_order.iter().filter_map(|id| self.get(id))
    }

    pub fn apply(&mut self, event: &Event) -> Applied {
        match event {
            Event::ItemStarted { item }
            | Event::ItemUpdated { item }
            | Event::ItemCompleted { item } => self.upsert(item),
            Event::ItemDelta {
                item, kind, data, ..
            } => self.delta(item, *kind, data),
            Event::TurnRetrying { dropped, .. } => {
                self.remove(dropped);
                Applied::Turn
            }
            Event::Rewound {
                generation,
                dropped,
                ..
            } => {
                self.generation = *generation;
                self.remove(dropped);
                Applied::History
            }
            Event::Compacted {
                generation,
                boundary,
                kept,
                summary,
            } => {
                self.generation = *generation;
                if let Some(cut) = self.model_order.iter().position(|id| id == boundary) {
                    self.cut = Some(boundary.clone());
                    splice_compaction(&mut self.model_order, cut, kept, summary);
                    self.model_ids = self.model_order.iter().cloned().collect();
                }
                Applied::History
            }
            _ => Applied::Nothing,
        }
    }

    /// A bounded client snapshot shares its retained bodies with the full one.
    pub fn window(&self, start: usize) -> Self {
        let items = self.items()[start..].to_vec();
        let positions = positions(&items);
        let model_order: Vec<_> = self
            .model_order
            .iter()
            .filter(|id| positions.contains_key(*id))
            .cloned()
            .collect();
        let model_ids = model_order.iter().cloned().collect();
        Self {
            generation: self.generation,
            items,
            positions,
            model_order,
            model_ids,
            hidden: 0,
            cut: None,
            has_more: self.has_more || start > 0,
        }
    }

    /// Combine storage's full client order with the live window at the same
    /// mailbox cut. Live bodies include deltas that are not durable frames.
    pub fn with_history(&self, mut history: Self) -> Self {
        for item in &mut history.items {
            if let Some(&at) = self.positions.get(&item.id) {
                *item = self.items[at].clone();
            }
        }
        history.model_order = self.model_order.clone();
        history.model_ids = self.model_ids.clone();
        history.generation = self.generation;
        history.cut = self.cut.clone();
        history.hidden = 0;
        history.has_more = false;
        history
    }

    fn upsert(&mut self, item: &Item) -> Applied {
        match self.positions.get(&item.id) {
            Some(&at) => self.items[at] = Arc::new(item.clone()),
            None => {
                self.positions.insert(item.id.clone(), self.items.len());
                self.items.push(Arc::new(item.clone()));
            }
        }
        // An authoritative update can reintroduce an item removed by a cut.
        if self.model_ids.insert(item.id.clone()) {
            self.model_order.push(item.id.clone());
        }
        Applied::Item(item.id.clone())
    }

    fn remove(&mut self, dropped: &[ItemId]) {
        let dropped: HashSet<_> = dropped.iter().collect();
        self.hidden -= self.items[..self.hidden]
            .iter()
            .filter(|item| dropped.contains(&item.id))
            .count();
        self.items.retain(|item| !dropped.contains(&item.id));
        self.positions = positions(&self.items);
        self.model_order.retain(|id| !dropped.contains(id));
        self.model_ids.retain(|id| !dropped.contains(id));
    }

    fn delta(&mut self, id: &ItemId, kind: DeltaKind, data: &str) -> Applied {
        let Some(&at) = self.positions.get(id) else {
            return Applied::Nothing;
        };
        if self.items[at].is_terminal() {
            return Applied::Nothing;
        }
        let target = Arc::make_mut(&mut self.items[at]);
        match (&mut target.body, kind) {
            (ItemBody::Assistant { text }, DeltaKind::Text) => text.push_str(data),
            (ItemBody::Reasoning { text, .. }, DeltaKind::Reasoning) => text.push_str(data),
            (ItemBody::ToolCall { progress, .. }, DeltaKind::Tail) => *progress = Some(data.into()),
            _ => return Applied::Nothing,
        }
        Applied::Item(id.clone())
    }
}

impl FromIterator<Item> for Transcript {
    fn from_iter<T: IntoIterator<Item = Item>>(items: T) -> Self {
        let mut transcript = Self::new();
        for item in items {
            transcript.apply(&Event::ItemCompleted { item });
        }
        transcript
    }
}

fn positions(items: &[Arc<Item>]) -> HashMap<ItemId, usize> {
    items
        .iter()
        .enumerate()
        .map(|(at, item)| (item.id.clone(), at))
        .collect()
}

/// Before the seam only kept ids survive; the summary precedes the untouched tail.
fn splice_compaction(order: &mut Vec<ItemId>, cut: usize, kept: &[ItemId], summary: &ItemId) {
    let summary = order
        .iter()
        .position(|id| id == summary)
        .map(|at| order.remove(at));
    let cut = cut.min(order.len());
    let (head, tail) = order.split_at(cut);
    let mut next: Vec<_> = head
        .iter()
        .filter(|id| kept.contains(id))
        .cloned()
        .collect();
    next.extend(summary);
    next.extend(tail.iter().cloned());
    *order = next;
}

impl Serialize for Transcript {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        use serde::ser::SerializeStruct;
        let mut wire = serializer.serialize_struct("Transcript", 2)?;
        wire.serialize_field("historyGeneration", &self.generation)?;
        wire.serialize_field(
            "items",
            &self.items().iter().map(AsRef::as_ref).collect::<Vec<_>>(),
        )?;
        wire.end()
    }
}

impl<'de> Deserialize<'de> for Transcript {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        #[derive(Deserialize)]
        #[serde(rename_all = "camelCase")]
        struct ClientSnapshot {
            #[serde(default)]
            history_generation: u64,
            items: Vec<Item>,
        }
        let wire = ClientSnapshot::deserialize(deserializer)?;
        let mut transcript: Self = wire.items.into_iter().collect();
        transcript.generation = wire.history_generation;
        Ok(transcript)
    }
}

#[cfg(test)]
mod tests;
