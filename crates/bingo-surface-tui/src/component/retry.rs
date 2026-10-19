//! A retry's wait is sampled from its frame, never counted down in state.
use crate::{
    clock::Now,
    layout::grid,
    transcript::Rows,
    views::{self, Drawing},
};
use bingo_sdk::{
    View,
    component::{Props, SpinnerProps},
    view::{TextChild, TextMotion, TextStyle},
};
use ratatui::text::Span;
use std::time::Duration;

fn props(rows: &Rows<'_>) -> Option<Props> {
    let retry = rows.state.turn.as_ref()?.retrying.as_ref()?;
    Some(Props::Spinner(SpinnerProps {
        word: retry.reason.clone(),
        message: None,
        suffix: String::new(),
        mode: "retry".into(),
        retry: Some(retry.clone()),
    }))
}

pub(super) fn draw(rows: &Rows<'_>) -> Drawing {
    let Some(props) = props(rows) else {
        return Drawing::default();
    };
    super::resolve(
        rows.state,
        rows.pulled,
        "Retry",
        &props,
        rows.width,
        &views::Marks::at(rows.now),
        |props, width| engine(props, width, rows.now),
    )
}

pub(super) fn request(rows: &Rows<'_>, frame: &mut crate::layout::Layout, height: u16) {
    let from = frame.blocks.closing_start();
    let to = from + draw(rows).lines.len();
    if frame
        .visible_lines(usize::from(height))
        .iter()
        .any(|range| to > range.start && from < range.end)
        && let Some(props) = props(rows)
        && let Some(key) = super::key(rows.state, "Retry", &props, rows.width as u16, height)
    {
        frame.pulls.push(key);
    }
}

fn text(value: String, ink: &str, motion: Option<TextMotion>) -> TextChild {
    TextChild::Node(Box::new(View::Text {
        children: vec![TextChild::String(value)],
        style: TextStyle {
            color: Some(ink.into()),
            motion: motion.map(Box::new),
            ..Default::default()
        },
    }))
}

fn engine(props: &Props, width: usize, now: Now) -> Drawing {
    let Props::Spinner(props) = props else {
        return Drawing::default();
    };
    let Some(retry) = &props.retry else {
        return Drawing::default();
    };
    let elapsed = now
        .wall
        .duration_since(retry.started_at)
        .max(jiff::SignedDuration::ZERO)
        .as_millis() as u64;
    let remaining = retry.delay_ms.saturating_sub(elapsed);
    let view = View::Text {
        children: vec![
            TextChild::String("   ".into()),
            text(
                crate::theme::glyphs::get("still").into(),
                "amber",
                Some(TextMotion::Spin),
            ),
            text(format!(" {}", props.word), "amber", None),
            text(crate::theme::glyphs::get("sep").into(), "dim", None),
            text(
                format!("retrying in {}s", remaining.div_ceil(1000)),
                "muted",
                None,
            ),
        ],
        style: Default::default(),
    };
    let mut drawing = views::marked(&view, width, &views::Marks::at(now));
    if remaining > 0 {
        let until = (remaining - 1) % 1000 + 1;
        drawing.next = drawing
            .next
            .into_iter()
            .chain(Some(now.instant + Duration::from_millis(until)))
            .min();
    }
    if let Some(line) = drawing.lines.first_mut() {
        let right = format!("attempt {} of {}", retry.attempt, retry.max);
        *line = grid::pad_to(line.clone(), width.saturating_sub(grid::RM + right.len()));
        line.spans.push(Span::styled(right, crate::theme::dim()));
    }
    drawing
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::*;
    #[test]
    fn countdown_is_ceiled_and_still_frames_wake_only_when_it_changes() {
        let mut state = folded(vec![
            frame(1, started("trn_1")),
            frame(
                2,
                bingo_sdk::Event::TurnRetrying {
                    turn: bingo_sdk::TurnId::from_raw("trn_1"),
                    attempt: 2,
                    max: 6,
                    delay_ms: 3000,
                    reason: "rate limited".into(),
                    dropped: vec![],
                },
            ),
        ]);
        let now = still(scene().1);
        state
            .turn
            .as_mut()
            .unwrap()
            .retrying
            .as_mut()
            .unwrap()
            .started_at = now.wall;
        let props = Props::Spinner(SpinnerProps {
            word: "rate limited".into(),
            message: None,
            suffix: String::new(),
            mode: "retry".into(),
            retry: state.turn.unwrap().retrying,
        });
        for (at, left, next) in [
            (0, 3, Some(1000)),
            (999, 3, Some(1000)),
            (1000, 2, Some(2000)),
            (3000, 0, None),
        ] {
            let drawing = engine(&props, 100, later(now, at));
            assert!(
                drawing.lines[0]
                    .to_string()
                    .contains(&format!("rate limited · retrying in {left}s"))
            );
            assert!(drawing.lines[0].to_string().ends_with("attempt 2 of 6"));
            assert_eq!(drawing.next, next.map(|at| later(now, at).instant));
            assert_eq!(engine(&props, 0, later(now, at)).next, drawing.next);
        }
    }
}
