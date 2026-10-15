//! Persistence of the journal. The actor appends before it publishes.

pub mod reader;

mod checkpoint;
pub use checkpoint::{CHECKPOINT_FORMAT, CHECKPOINT_VERSION, Checkpoint, Resumed};

use async_trait::async_trait;

use crate::error::KernelError;
use crate::event::{Frame, SessionSummary};
use crate::host::SessionFilter;
use crate::ids::{Seq, SessionId};

/// An implementation does blocking I/O on the blocking pool.
#[async_trait]
pub trait SessionStore: Send + Sync {
    async fn create(&self, summary: &SessionSummary) -> Result<(), KernelError>;

    async fn append(&self, session: &SessionId, frame: &Frame) -> Result<(), KernelError>;

    /// Durable frames with `seq > since`, in order.
    async fn replay(&self, session: &SessionId, since: Seq) -> Result<Vec<Frame>, KernelError>;

    /// Stores without checkpoints remain correct through full replay.
    async fn checkpoint(
        &self,
        _session: &SessionId,
        _checkpoint: &Checkpoint,
    ) -> Result<(), KernelError> {
        Ok(())
    }

    async fn resume(&self, session: &SessionId) -> Result<Resumed, KernelError> {
        Ok(Resumed {
            state: None,
            tail: self.replay(session, Seq::ZERO).await?,
        })
    }

    async fn list(&self, filter: &SessionFilter) -> Result<Vec<SessionSummary>, KernelError>;

    /// Read one summary without acquiring ownership or repairing files.
    async fn summary(&self, session: &SessionId) -> Result<SessionSummary, KernelError> {
        let frames = self.replay(session, Seq::ZERO).await?;
        let summary = frames
            .iter()
            .find_map(|frame| match &frame.event {
                crate::Event::SessionUpdated { summary } => Some(summary.clone()),
                _ => None,
            })
            .ok_or_else(|| {
                KernelError::new(crate::ErrorCode::SessionNotFound, "no stored summary")
            })?;
        let mut state = crate::SessionState::new(summary);
        for frame in frames {
            state.apply(&frame);
        }
        Ok(state.summary)
    }

    /// Change only the title under a temporary exclusive claim, preserving
    /// activity time. `None` clears it, so lists fall back to the first ask.
    async fn rename(&self, _session: &SessionId, _title: Option<&str>) -> Result<(), KernelError> {
        Err(KernelError::new(
            crate::ErrorCode::Internal,
            "this store cannot rename sessions",
        ))
    }

    /// Observe ownership without claiming an unowned session. A store that
    /// cannot tell refuses, so no caller mistakes "unknown" for "free".
    async fn locked(&self, _session: &SessionId) -> Result<bool, KernelError> {
        Err(KernelError::new(
            crate::ErrorCode::Internal,
            "this store cannot tell whether a session is open elsewhere",
        ))
    }

    async fn delete(&self, session: &SessionId) -> Result<(), KernelError>;

    /// Take the session for this process, from create or resume until
    /// `release` or exit; a second holder gets `SessionLocked` (ADR-0005).
    async fn acquire(&self, _session: &SessionId) -> Result<(), KernelError> {
        Ok(())
    }

    async fn release(&self, _session: &SessionId) -> Result<(), KernelError> {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Only what a store must write; every optional verb is left defaulted.
    struct Minimal;

    #[async_trait]
    impl SessionStore for Minimal {
        async fn create(&self, _: &SessionSummary) -> Result<(), KernelError> {
            Ok(())
        }
        async fn append(&self, _: &SessionId, _: &Frame) -> Result<(), KernelError> {
            Ok(())
        }
        async fn replay(&self, _: &SessionId, _: Seq) -> Result<Vec<Frame>, KernelError> {
            Ok(vec![])
        }
        async fn list(&self, _: &SessionFilter) -> Result<Vec<SessionSummary>, KernelError> {
            Ok(vec![])
        }
        async fn delete(&self, _: &SessionId) -> Result<(), KernelError> {
            Ok(())
        }
    }

    #[tokio::test]
    async fn a_store_that_cannot_tell_refuses_like_rename() {
        let id = SessionId::from_raw("ses_any");
        let locked = Minimal.locked(&id).await.expect_err("unknown is not free");
        let renamed = Minimal.rename(&id, Some("x")).await.expect_err("refused");
        assert_eq!(locked.code, renamed.code);
    }
}
