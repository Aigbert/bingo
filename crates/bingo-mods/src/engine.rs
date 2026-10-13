//! The host-side handle never executes JavaScript.
use std::{path::Path, sync::Arc, time::Duration};
use serde_json::Value;

#[derive(Clone, Debug)]
pub struct Limits {
    pub memory_bytes: usize,
    pub stack_bytes: usize,
    pub hook_time: Duration,
}
impl Default for Limits {
    fn default() -> Self {
        Self { memory_bytes: 64 * 1024 * 1024, stack_bytes: 512 * 1024, hook_time: Duration::from_secs(10) }
    }
}

#[derive(Debug, thiserror::Error)]
#[error("{kind}: {message}")]
pub struct Fault {
    pub kind: &'static str,
    pub message: String,
}

pub struct Engine;
impl Engine {
    pub async fn start(_root: &Path, _module: &str, _config: Value, _limits: Limits) -> Result<Arc<Self>, Fault> {
        Err(Fault { kind: "load", message: "module thread is not implemented".into() })
    }
}
