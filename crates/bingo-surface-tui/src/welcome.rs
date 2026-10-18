//! The wordmark and the facts of this session, at the head of its transcript.

use bingo_sdk::{Driver, SessionState, SessionSummary};

/// Host answers shown at the opening, independent of session state.
#[derive(Debug, Default)]
pub struct Details {
    pub update: Option<String>,
    pub resume: Option<SessionSummary>,
}

#[derive(Clone, Copy, Default)]
pub struct Entrance<'a> {
    pub resume: Option<&'a SessionSummary>,
}

/// Whether this surface opened the session, and so has a box to say things
/// in. The start-up check asks the same question before it asks anything of
/// the network: a run with nowhere to put an answer does not go looking.
pub fn wanted(state: &SessionState) -> bool {
    state.summary.parent.is_none() && state.summary.driver == Driver::Model
}
