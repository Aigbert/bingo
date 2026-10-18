//! Three bands: stream, dock, footer; geometry contains no session facts.

use ratatui::layout::Rect;

/// The rule and one row to type in.
pub const COMPOSER_MIN: u16 = 3;
/// The rule and ten rows of draft.
pub const COMPOSER_MAX: u16 = 11;

/// What the frame must make room for, measured before the geometry is cut.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Demand {
    /// Rows of text in the input box, before its border.
    pub composer: u16,
    /// Rows of thumbnails standing on the box, when the draft carries
    /// pictures a terminal can draw (M48; outside the border since 2026-09-04).
    pub strip: u16,
    /// Rows between the transcript and the box: the activity row and whatever
    /// is queued behind it.
    pub activity: u16,
    /// Rows of the open palette, under the box; none while it is closed.
    pub palette: u16,
}

/// The frame, top to bottom. Every region may be empty; the status line and
/// the input box are the last to go, and an open palette takes the status
/// line's and the hint row's place.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Regions {
    pub transcript: Rect,
    pub activity: Rect,
    /// The thumbnails' rows, directly on the box's top border.
    pub strip: Rect,
    pub composer: Rect,
    pub status: Rect,
    /// The open palette, under the box.
    pub palette: Rect,
}

impl Regions {
    /// Everything above the input box: what a layer may cover.
    pub fn above(&self) -> Rect {
        // `union` of an empty rect with another is not the other — an empty
        // transcript would drag the band back to row 0 with no height — so
        // only the regions with rows are joined.
        [self.transcript, self.activity, self.strip]
            .into_iter()
            .filter(|region| region.height > 0)
            .reduce(|band, region| band.union(region))
            .unwrap_or_default()
    }
}

/// Cut the frame into its regions from the bottom up.
pub fn regions(size: Rect, demand: Demand) -> Regions {
    let mut rest = size;
    let status = take_bottom(&mut rest, 1);
    let rows = palette_rows(demand.palette, rest.height);
    let palette = take_bottom(&mut rest, rows);
    let rows = composer_rows(demand.composer, rest.height)
        .min(crate::layout::grid::dock_cap(usize::from(size.height)) as u16 + 2);
    let composer = take_bottom(&mut rest, rows);
    let rows = strip_rows(demand.strip, rest.height);
    let strip = take_bottom(&mut rest, rows);
    let rows = activity_rows(demand.activity, rest.height);
    let activity = take_bottom(&mut rest, rows);
    Regions {
        transcript: rest,
        activity,
        strip,
        composer,
        status,
        palette,
    }
}

/// The palette leaves the box its smallest self above it.
fn palette_rows(want: u16, room: u16) -> u16 {
    want.min(room.saturating_sub(COMPOSER_MIN))
}

/// The box grows with the draft and stops at ten rows; it never shrinks below
/// one row, and it takes what is left when even that does not fit.
fn composer_rows(text: u16, room: u16) -> u16 {
    text.saturating_add(2)
        .clamp(COMPOSER_MIN, COMPOSER_MAX)
        .min(room)
}

/// The strip is rows of its own above the box, never rows out of it: a
/// picture pasted into a full draft must not push a line of it off the
/// screen. It is all there or not there — a thumbnail cut to one row is not a
/// thumbnail — and it yields before the transcript's last row does.
fn strip_rows(want: u16, room: u16) -> u16 {
    if want < room { want } else { 0 }
}

/// The activity rows yield before the transcript's last row does.
fn activity_rows(want: u16, room: u16) -> u16 {
    want.min(room.saturating_sub(1))
}

/// Take `height` rows off the bottom of `rest`, or as many as there are.
fn take_bottom(rest: &mut Rect, height: u16) -> Rect {
    let height = height.min(rest.height);
    rest.height -= height;
    Rect {
        y: rest.bottom(),
        height,
        ..*rest
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn three_bands_stack_at_every_size() {
        for width in [1, 20, 80, 100] {
            for height in 1..50 {
                let size = Rect::new(0, 0, width, height);
                let r = regions(
                    size,
                    Demand {
                        composer: 1,
                        ..Demand::default()
                    },
                );
                assert_eq!(r.status.bottom(), height);
                assert_eq!(r.status.height, 1);
                assert_eq!(r.composer.bottom(), r.status.y);
                assert_eq!(r.transcript.bottom(), r.composer.y);
                assert_eq!(r.composer.height, COMPOSER_MIN.min(height - 1));
            }
        }
    }
    #[test]
    fn the_dock_caps_its_body_and_palette_keeps_its_place() {
        let at = |demand| regions(Rect::new(0, 0, 80, 24), demand);
        let r = at(Demand {
            composer: 40,
            ..Demand::default()
        });
        assert_eq!(r.composer.height, COMPOSER_MAX);
        let r = at(Demand {
            composer: 1,
            palette: 8,
            ..Demand::default()
        });
        assert_eq!(r.composer.bottom(), r.palette.y);
        assert_eq!(r.palette.bottom(), r.status.y);
        assert_eq!(r.status.height, 1);
    }
    #[test]
    fn thumbnails_stand_above_the_dock_and_yield_whole() {
        let at = |height| {
            regions(
                Rect::new(0, 0, 80, height),
                Demand {
                    composer: 1,
                    strip: 3,
                    ..Demand::default()
                },
            )
        };
        assert_eq!(at(8).strip.height, 3);
        assert_eq!(at(7).strip.height, 0);
        assert_eq!(at(8).strip.bottom(), at(8).composer.y);
    }
}

#[cfg(test)]
mod m148 {
    use crate::test_support::*;
    use bingo_sdk::{
        View,
        slot::{self, Slot},
    };
    use serde_json::json;

    #[test]
    fn m148_unplaced_payloads_leave_the_screen_unchanged_at_every_width() {
        for width in [80, 120, 200] {
            for payload in [
                as_payload(board_view()),
                progress_view(4, 10),
                slot::with(
                    json!({}),
                    [Slot::Rail {
                        title: "legacy".into(),
                        view: View::text("unplaced").into(),
                    }],
                ),
                slot::with(
                    json!({}),
                    [Slot::Panel {
                        title: "legacy".into(),
                        view: View::text("unplaced").into(),
                    }],
                ),
            ] {
                let (mut ui, now) = scene();
                let before = draw_sized(width, 40, &state(), &mut ui, now);
                let state = folded(vec![
                    frame(1, extended("stranger", "legacy", payload.clone())),
                    frame(2, signalled("stranger", "legacy", payload)),
                ]);
                let after = draw_sized(width, 40, &state, &mut ui, now);
                assert_eq!(ui.shown.regions.transcript.width, width);
                assert_eq!(after, before);
            }
        }
    }

    #[test]
    fn m148_room_and_wake_data_draw_nothing_at_every_width() {
        for width in [80, 120, 200] {
            let (mut ui, now) = scene();
            let before = draw_sized(width, 40, &state(), &mut ui, now);
            let state = folded(vec![
                frame(
                    1,
                    signalled(
                        "bingo.rooms",
                        "owed",
                        json!({"kind":"table", "headers":["room","owed"], "rows":[["#design","scout"]]}),
                    ),
                ),
                frame(
                    2,
                    extended(
                        "bingo.rooms",
                        "members",
                        json!({"kind":"text", "text":"scout"}),
                    ),
                ),
                frame(
                    3,
                    extended(
                        "bingo.schedule",
                        "wake",
                        json!({"at":"2030-01-01T00:00:00Z"}),
                    ),
                ),
            ]);
            assert_eq!(draw_sized(width, 40, &state, &mut ui, now), before);
        }
    }
}

#[cfg(test)]
mod m183 {
    use crate::test_support::*;
    use bingo_sdk::{ContextUsage, Effort, Event, ItemStatus};

    #[test]
    fn prototype_frame_idle_running_done_and_narrow() {
        let mut state = state();
        state.summary.id = bingo_sdk::SessionId::from_raw("ses_3f2a");
        state.summary.cwd = crate::paths::home()
            .map(|home| format!("{home}/src/demo"))
            .unwrap_or_else(|| "~/src/demo".into());
        state.summary.title = Some("label the total in main".into());
        state.summary.model = Some("claude-sonnet-5-5".into());
        state.summary.effort = Some(Effort::Medium);
        state.context = Some(ContextUsage {
            parts: None,
            used: 14_000,
            window: 200_000,
            trigger: 180_000,
        });
        state
            .extensions
            .entry("_bingo.context".into())
            .or_default()
            .insert(
                "location".into(),
                bingo_sdk::slot::with(
                    serde_json::json!({}),
                    [bingo_sdk::slot::Slot::Location {
                        cwd: state.summary.cwd.clone(),
                        branch: Some("main".into()),
                    }],
                ),
            );
        let (mut ui, now) = scene();
        for (name, events) in [
            ("idle", vec![]),
            (
                "running",
                vec![
                    started("turn_burrowing"),
                    Event::ItemStarted {
                        item: user("ask", "label the total in main"),
                    },
                ],
            ),
            (
                "done",
                vec![
                    Event::ItemCompleted {
                        item: assistant(
                            "answer",
                            "Done. main now prints a labelled total.",
                            ItemStatus::Completed,
                        ),
                    },
                    completed("turn_burrowing", bingo_sdk::TurnStatus::Completed),
                ],
            ),
        ] {
            for event in events {
                state.apply(&frame(state.seq.0 + 1, event));
            }
            for (width, height) in [(108, 30), (80, 24)] {
                insta::assert_snapshot!(
                    format!("frame_{name}_{width}x{height}"),
                    draw_sized(width, height, &state, &mut ui, later(now, 3_000))
                );
            }
        }
    }
}
