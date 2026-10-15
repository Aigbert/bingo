//! The wordmark and the facts of this session, at the head of its transcript.
use crate::{paths, theme};
use bingo_sdk::{Driver, SessionState, SessionSummary};
use jiff::Timestamp;
use ratatui::text::{Line, Span};

/// The site's cell grid, paired into half-block rows.
const WORDMARK: [&str; 4] = [
    "█      ▀",
    "█▀▀▀▀█ █ █▀▀▀▀█ █▀▀▀▀█ █▀▀▀▀█",
    "█▄▄▄▄█ █ █    █ █▄▄▄▄█ █▄▄▄▄█",
    "                ▄▄▄▄▄█",
];

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

/// Everything the opening is decided from, read once when the surface has its
/// session and its screen (design §11).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Opens {
    /// This surface opened the session: a top-level model session, and not a
    /// sub-session or a room ([`wanted`]).
    pub ours: bool,
    /// Nothing has happened in it yet. A resumed session carries its history,
    /// and a session that already happened is not entered.
    pub fresh: bool,
    /// Work came in on the command line. A run that was given something to do
    /// goes and does it.
    pub asked: bool,
    /// How big the terminal is, in columns and rows.
    pub screen: (u16, u16),
    /// The look draws twenty-four bits.
    pub colour: bool,
    /// It draws more than ASCII.
    pub glyphs: bool,
    /// This run animates at all (`BINGO_MOTION`).
    pub motion: bool,
}

/// The smallest screen the opening plays on: under this there is no room for a
/// world above the composer, and the box is drawn as it always was.
pub const ROOM: (u16, u16) = (40, 16);

/// Whether the opening plays. Every reason it would not is a fact about this
/// run, and they are all here: nothing about the piece itself decides it.
pub fn opens(at: Opens) -> bool {
    at.ours
        && at.fresh
        && !at.asked
        && at.colour
        && at.glyphs
        && at.motion
        && at.screen.0 >= ROOM.0
        && at.screen.1 >= ROOM.1
}

/// The wordmark's rows are held through every frame of the opening.
pub fn draw(
    state: &SessionState,
    width: usize,
    entrance: Entrance<'_>,
    update: Option<&str>,
    now: Timestamp,
) -> Vec<Line<'static>> {
    if !wanted(state) {
        return Vec::new();
    }
    let mut rows = mark(width, entrance.height);
    rows.push(Line::from(Span::styled(facts(state, width), theme::dim())));
    rows.push(Line::from(Span::styled(
        resume(entrance.resume, width, now),
        theme::dim(),
    )));
    if let Some(version) = update {
        rows.push(Line::from(Span::styled(
            crate::views::clip(&format!("↑ v{version} is out · bingo update"), width),
            theme::presence(),
        )));
    }
    rows
}

fn mark(width: usize, height: u16) -> Vec<Line<'static>> {
    if width < 40 || height < 16 || theme::glyphs() == &theme::ASCII {
        return vec![Line::from(vec![
            Span::styled("bingo", theme::text()),
            Span::styled(theme::mark(), theme::presence()),
        ])];
    }
    WORDMARK
        .iter()
        .enumerate()
        .map(|(row, text)| {
            let mut spans = vec![Span::styled(*text, theme::text())];
            if row == 2 {
                spans.push(Span::styled(
                    format!(" {}", theme::mark()),
                    theme::presence(),
                ));
            }
            Line::from(spans)
        })
        .collect()
}

fn facts(state: &SessionState, width: usize) -> String {
    let mut parts: Vec<String> = state.summary.model.clone().into_iter().collect();
    if let Some(level) = state
        .config
        .kernel
        .get("thinking")
        .and_then(|value| serde_json::from_value::<bingo_sdk::Effort>(value.clone()).ok())
    {
        parts.push(level.name().into());
    }
    parts.extend(bingo_sdk::slot::placed(state).into_iter().filter_map(
        |placed| match placed.slot {
            bingo_sdk::slot::Slot::Mode { current, .. } => Some(current),
            _ => None,
        },
    ));
    let prefix = parts.join(" · ");
    let room = width.saturating_sub(
        unicode_width::UnicodeWidthStr::width(prefix.as_str())
            + if prefix.is_empty() { 0 } else { 3 },
    );
    let cwd = paths::short(&state.summary.cwd, "", paths::home());
    parts.push(paths::elide(&cwd, room));
    crate::views::clip(&parts.join(" · "), width)
}

fn resume(previous: Option<&SessionSummary>, width: usize, now: Timestamp) -> String {
    let Some(previous) = previous else {
        return String::new();
    };
    let arrow = if theme::glyphs() == &theme::ASCII {
        "<"
    } else {
        "↺"
    };
    let title = previous.title.as_deref().unwrap_or("previous session");
    let suffix = format!(" · {} · /resume", bingo_sdk::ago(now, previous.updated_at));
    let room = width.saturating_sub(suffix.chars().count() + 2);
    crate::views::clip(
        &format!("{arrow} {}{suffix}", crate::views::clip(title, room)),
        width,
    )
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
        let at = Entrance {
            height: 14,
            resume: None,
        };
        let empty = draw(&state, 80, at, None, state.summary.updated_at);
        assert_eq!(empty[0].to_string(), "bingo▍");
        let filled = draw(
            &state,
            80,
            Entrance {
                resume: Some(&state.summary),
                ..at
            },
            None,
            state.summary.updated_at,
        );
        assert_eq!(empty.len(), filled.len());
        assert!(filled[2].to_string().contains("/resume"));
    }
    /// The table of §11: every reason the opening does not play.
    #[test]
    fn the_opening_plays_on_a_fresh_session_and_nowhere_else() {
        let plays = Opens {
            ours: true,
            fresh: true,
            asked: false,
            screen: ROOM,
            colour: true,
            glyphs: true,
            motion: true,
        };
        assert!(opens(plays), "a fresh session on a terminal with room");
        for (why, at) in [
            (
                "a sub-session or a room",
                Opens {
                    ours: false,
                    ..plays
                },
            ),
            (
                "a session that was resumed",
                Opens {
                    fresh: false,
                    ..plays
                },
            ),
            (
                "a run given work on the command line",
                Opens {
                    asked: true,
                    ..plays
                },
            ),
            (
                "eight colours",
                Opens {
                    colour: false,
                    ..plays
                },
            ),
            (
                "ASCII alone",
                Opens {
                    glyphs: false,
                    ..plays
                },
            ),
            (
                "motion turned off",
                Opens {
                    motion: false,
                    ..plays
                },
            ),
            (
                "a narrow terminal",
                Opens {
                    screen: (ROOM.0 - 1, ROOM.1),
                    ..plays
                },
            ),
            (
                "a short terminal",
                Opens {
                    screen: (ROOM.0, ROOM.1 - 1),
                    ..plays
                },
            ),
        ] {
            assert!(!opens(at), "it should not play on {why}");
        }
        assert!(
            opens(Opens {
                screen: (200, 60),
                ..plays
            }),
            "and a large terminal is still a terminal"
        );
    }

    #[test]
    fn a_session_this_surface_did_not_open_is_not_welcomed() {
        let mut child = state();
        child.summary = child_summary("reviewer");
        assert!(!wanted(&child), "a sub-session joins");
        assert!(lines(&child, 60, None).is_empty());

        let mut room = folded(vec![]);
        room.summary = log_summary("#design");
        assert!(!wanted(&room), "a room is not a session of ours");
        assert!(lines(&room, 60, Some("0.5.0")).is_empty());
    }
}
