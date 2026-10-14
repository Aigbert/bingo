//! Read durable state without acquiring or activating a session.

use async_trait::async_trait;

use crate::{KernelError, ServiceKey, SessionState, SessionSummary};

pub static KEY: ServiceKey<dyn SessionReader> = ServiceKey::new("bingo.sessions.reader");

/// A store's optional in-process read face. Reads must neither acquire the
/// session nor repair or append to its journal. The snapshot contains durable
/// frames only; live callers needing mailbox ordering must still use `open`.
#[async_trait]
pub trait SessionReader: Send + Sync {
    async fn read(&self, summary: &SessionSummary) -> Result<SessionState, KernelError>;
}
