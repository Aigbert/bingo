//! Session-directory facts and the client's minted continuation.
use crate::{Action, SessionId, SessionSummary};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum SessionsScope {
    #[default]
    Here,
    All,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct SessionsProps {
    pub session: SessionId,
    pub cwd: String,
    pub query: String,
    pub scope: SessionsScope,
    pub limit: usize,
    /// One extra summary is the sentinel for the next page.
    pub sessions: Vec<SessionSummary>,
    pub current: SessionId,
    #[schemars(with = "String")]
    pub now: jiff::Timestamp,
    pub action: Action,
}
