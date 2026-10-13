//! Persistence of the journal. The actor appends before it publishes.

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
