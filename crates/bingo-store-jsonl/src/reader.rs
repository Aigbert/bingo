//! The service reader never enters the writer or the ownership protocol.

use async_trait::async_trait;
use bingo_sdk::store::reader::SessionReader;
use bingo_sdk::{KernelError, Seq, SessionState, SessionSummary};

use crate::JsonlStore;

#[async_trait]
impl SessionReader for JsonlStore {
    async fn read(&self, summary: &SessionSummary) -> Result<SessionState, KernelError> {
        let store = self.clone();
        let summary = summary.clone();
        crate::off_runtime(move || {
            let mut state = SessionState::new(summary.clone());
            for frame in crate::journal::replay(&store.existing(&summary.id)?, Seq::ZERO)? {
                state.apply(&frame);
            }
            Ok(state)
        })
        .await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bingo_sdk::SessionStore;

    #[tokio::test]
    async fn reading_a_held_session_neither_writes_nor_takes_ownership() {
        let (_root, owner) = crate::tests::root_with();
        let summary = crate::tests::summary();
        owner.create(&summary).await.unwrap();
        owner.acquire(&summary.id).await.unwrap();
        let frame = crate::tests::frame(
            1,
            bingo_sdk::Event::SessionUpdated {
                summary: summary.clone(),
            },
        );
        owner.append(&summary.id, &frame).await.unwrap();
        let journal = owner.root().join(summary.id.as_str()).join("journal.jsonl");
        let before = std::fs::read(&journal).unwrap();
        let reader = JsonlStore::new(owner.root());
        let state = reader.read(&summary).await.unwrap();
        assert_eq!(state.summary, summary);
        assert_eq!(state.seq, frame.seq);
        assert_eq!(std::fs::read(&journal).unwrap(), before);
        assert!(!journal.with_file_name("checkpoint.json").exists());
        assert!(reader.acquire(&summary.id).await.is_err());
        owner.release(&summary.id).await.unwrap();
        reader.acquire(&summary.id).await.unwrap();
    }
}
