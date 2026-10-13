//! The same page and pin rules over a live window or a stored transcript.
use super::fnv1a64;
use bingo_sdk::*;
use std::sync::Arc;

pub(super) struct History<'a>(pub &'a Transcript);
impl History<'_> {
    pub(super) fn history(&self, page: &HistoryPage) -> Result<HistoryChunk, KernelError> {
        self.check_generation(page.generation)?;
        let items = self.0.items();
        let end = match page.before.as_ref() {
            Some(before) => match items.iter().position(|item| &item.id == before) {
                Some(index) => index,
                None if page.max_bytes.is_some() => {
                    return Err(KernelError::new(
                        ErrorCode::StaleGeneration,
                        "the history cursor is no longer present",
                    ));
                }
                None => items.len(), // Legacy unbounded callers keep their old behavior.
            },
            None => items.len(),
        };
        let limit = if page.limit == 0 { end } else { page.limit };
        let Some(max_bytes) = page.max_bytes else {
            let start = end.saturating_sub(limit);
            return Ok(HistoryChunk {
                items: items[start..end]
                    .iter()
                    .map(|item| item.as_ref().clone())
                    .collect(),
                next: (start > 0).then(|| items[start].id.clone()),
                generation: self.0.generation(),
                oversized: None,
            });
        };
        self.bounded_history(items, end, limit, max_bytes)
    }

    pub(super) fn bounded_history(
        &self,
        items: &[Arc<Item>],
        end: usize,
        limit: usize,
        max_bytes: usize,
    ) -> Result<HistoryChunk, KernelError> {
        let mut start = end;
        let mut used = 256usize; // JSON-RPC envelope, generation and cursor.
        while start > 0 && end - start < limit {
            let size = serde_json::to_vec(items[start - 1].as_ref())
                .map_err(|error| {
                    KernelError::new(
                        ErrorCode::Internal,
                        format!("item cannot serialize: {error}"),
                    )
                })?
                .len();
            if used.saturating_add(size + 1) > max_bytes {
                break;
            }
            used += size + 1;
            start -= 1;
        }
        let slice = &items[start..end];
        let oversized = if slice.is_empty() && start > 0 {
            let item = &items[start - 1];
            let raw = serde_json::to_vec(item.as_ref()).map_err(|error| {
                KernelError::new(
                    ErrorCode::Internal,
                    format!("item cannot serialize: {error}"),
                )
            })?;
            Some(OversizedItem {
                id: item.id.clone(),
                total_bytes: raw.len(),
                checksum: fnv1a64(&raw),
            })
        } else {
            None
        };
        Ok(HistoryChunk {
            items: slice.iter().map(|item| item.as_ref().clone()).collect(),
            next: (start > 0)
                .then(|| {
                    slice
                        .first()
                        .map(|item| &item.id)
                        .or_else(|| oversized.as_ref().map(|item| &item.id))
                })
                .flatten()
                .cloned(),
            generation: self.0.generation(),
            oversized,
        })
    }

    pub(super) fn check_generation(&self, generation: Option<u64>) -> Result<(), KernelError> {
        if generation.is_some_and(|generation| generation != self.0.generation()) {
            return Err(KernelError::new(
                ErrorCode::StaleGeneration,
                "history changed",
            ));
        }
        Ok(())
    }

    pub(super) fn item_for_pin(&self, id: &ItemId, generation: u64) -> Result<Item, KernelError> {
        self.check_generation(Some(generation))?;
        self.0
            .items()
            .iter()
            .find(|item| &item.id == id)
            .map(|item| item.as_ref().clone())
            .ok_or_else(|| {
                KernelError::new(
                    ErrorCode::StaleGeneration,
                    format!("item {id} is no longer present"),
                )
            })
    }
}
