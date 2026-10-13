//! Derived persisted state, independently versioned from the client snapshot.

use crate::{Seq, SessionState};
use serde::{Deserialize, Serialize};

pub const CHECKPOINT_FORMAT: &str = "bingo-checkpoint";
pub const CHECKPOINT_VERSION: u32 = 3;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Checkpoint {
    pub format: String,
    pub version: u32,
    pub seq: Seq,
    pub offset: u64,
    pub line_hash: String,
    #[serde(with = "state")]
    pub state: SessionState,
}

impl Checkpoint {
    /// The store fills the byte witness for this state's durable sequence.
    pub fn new(state: SessionState) -> Self {
        Self {
            format: CHECKPOINT_FORMAT.into(),
            version: CHECKPOINT_VERSION,
            seq: state.seq,
            offset: 0,
            line_hash: String::new(),
            state,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Resumed {
    pub state: Option<SessionState>,
    pub tail: Vec<crate::Frame>,
}

mod state;

#[cfg(test)]
mod tests;
