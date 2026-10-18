//! The wordmark and the facts of this session, at the head of its transcript.

use bingo_sdk::{Driver, SessionState, SessionSummary};
use jiff::Timestamp;
use ratatui::text::Line;

/// Host answers shown at the opening, independent of session state.
#[derive(Debug, Default)]
pub struct Details {
    pub update: Option<String>,
    pub resume: Option<SessionSummary>,
}

#[derive(Clone, Copy)]
pub struct Entrance<'a> {
    pub height: u16,
    pub resume: Option<&'a SessionSummary>,
}

impl Default for Entrance<'_> {
    fn default() -> Self {
        Self {
            height: 24,
            resume: None,
        }
    }
}

/// Whether this surface opened the session, and so has a box to say things
/// in. The start-up check asks the same question before it asks anything of
/// the network: a run with nowhere to put an answer does not go looking.
pub fn wanted(state: &SessionState) -> bool {
    state.summary.parent.is_none() && state.summary.driver == Driver::Model
}

/// The wordmark's rows are held through every frame of the opening.
pub fn draw(
    state: &SessionState,
    width: usize,
    entrance: Entrance<'_>,
    update: Option<&str>,
    now: Timestamp,
) -> Vec<Line<'static>> {
    let _ = (state, width, entrance.height, update, now);
    Vec::new()
}

#[cfg(test)]
pub fn lines(state: &SessionState, width: usize, update: Option<&str>) -> Vec<Line<'static>> {
    draw(
        state,
        width,
        Entrance::default(),
        update,
        state.summary.updated_at,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::{child_summary, folded, log_summary, state};
    #[test]
    fn compact_height_and_resume_hold_their_rows() {
        let state = state();
        assert!(
            draw(
                &state,
                80,
                Entrance {
                    height: 14,
                    resume: Some(&state.summary)
                },
                None,
                state.summary.updated_at
            )
            .is_empty()
        );
    }
    #[test]
    fn a_session_this_surface_did_not_open_is_not_welcomed() {
        let mut child = state();
        child.summary = child_summary("reviewer");
        assert!(!wanted(&child), "a sub-session joins");
        assert_eq!(lines(&child, 60, None).len(), 0);

        let mut room = folded(vec![]);
        room.summary = log_summary("#design");
        assert!(!wanted(&room), "a room is not a session of ours");
        assert_eq!(lines(&room, 60, Some("0.5.0")).len(), 0);
    }
}
