//! Checkpoint-only codec; the public client snapshot stays flat.

use super::*;

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct Stored {
    history_generation: u64,
    items: Vec<Item>,
    model_order: Vec<ItemId>,
    hidden: usize,
    cut: Option<ItemId>,
    has_more: bool,
}

impl From<&Transcript> for Stored {
    fn from(value: &Transcript) -> Self {
        Self {
            history_generation: value.generation,
            items: value
                .items
                .iter()
                .map(|item| item.as_ref().clone())
                .collect(),
            model_order: value.model_order.clone(),
            hidden: value.hidden,
            cut: value.cut.clone(),
            has_more: value.has_more,
        }
    }
}

impl Stored {
    pub(crate) fn restore(self) -> Result<Transcript, &'static str> {
        let items: Vec<_> = self.items.into_iter().map(Arc::new).collect();
        let positions = positions(&items);
        let model_ids: HashSet<_> = self.model_order.iter().cloned().collect();
        if self.hidden > items.len()
            || positions.len() != items.len()
            || model_ids.len() != self.model_order.len()
            || model_ids.iter().any(|id| !positions.contains_key(id))
            || items[..self.hidden]
                .iter()
                .any(|item| !model_ids.contains(&item.id))
            || (self.hidden > 0 && !self.has_more)
        {
            return Err("invalid checkpoint transcript");
        }
        Ok(Transcript {
            generation: self.history_generation,
            items,
            positions,
            model_order: self.model_order,
            model_ids,
            hidden: self.hidden,
            cut: self.cut,
            has_more: self.has_more,
            revision: Default::default(),
        })
    }
}
