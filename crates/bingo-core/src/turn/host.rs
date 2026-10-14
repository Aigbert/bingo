//! The session actor as the turn sees it: what the turn loop may tell it and
//! ask of it, and nothing of how it is answered.

use std::sync::Arc;

use async_trait::async_trait;
use bingo_sdk::*;

/// The session actor as the turn sees it: a place to publish, a way to ask, a queue to absorb.
#[async_trait]
pub trait TurnHost: Send + Sync {
    /// A bridged after-tool observer requested completion at this round's boundary.
    fn stopped(&self) -> bool {
        false
    }
    fn emit(&self, event: Event);
    /// Return the canonical receipt, including one already recorded on resolution.
    async fn receipt(&self, item: Item) -> Result<Item, KernelError> {
        self.emit(Event::ItemCompleted { item: item.clone() });
        Ok(item)
    }
    /// The tools this turn resolved when it started (ADR-0009 §1): the offer
    /// every one of its requests carries. The session keeps them so that a
    /// call handed in from outside the turn is served by the very set the
    /// model was given (ADR-0036 §2).
    fn offered(&self, tools: Vec<Arc<dyn Tool>>);
    /// Whether the model's stream is open: the only time a call handed in
    /// from outside the model is let into the turn (ADR-0036 §2). Once it
    /// ends, the turn decides on what it heard and runs no such call again.
    fn streaming(&self, _open: bool) {}
    async fn ask(
        &self,
        item: Option<ItemId>,
        kind: InteractionKind,
        answers: Vec<AnswerSpec>,
    ) -> Result<Answer, KernelError>;
    /// Queued inputs eligible for steering, removed from the queue.
    async fn absorb(&self) -> Vec<(IntentId, Input)>;
}
