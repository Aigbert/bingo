//! The live row is a Spinner site at the end of the visible transcript.
use crate::{clock::Now, theme, transcript::Rows, views};
use bingo_sdk::{
    Item, ItemBody, ItemStatus, View,
    component::{Props, SpinnerProps},
    view::{TextChild, TextMotion, TextStyle},
};
use ratatui::text::{Line, Span};
use std::time::Instant;
use unicode_width::UnicodeWidthStr;

struct Row<'a> {
    props: Props,
    thought: Option<&'a Item>,
}

fn prepare<'a>(rows: &Rows<'a>) -> (Option<Row<'a>>, Option<Instant>) {
    let Some(turn) = &rows.state.turn else {
        return (None, None);
    };
    let items = rows
        .turns
        .get(&Some(turn.id.clone()))
        .map(Vec::as_slice)
        .unwrap_or_default();
    let thought =
        items.iter().rev().copied().find(|item| {
            matches!(item.body, ItemBody::Reasoning { .. }) && !item.status.is_terminal()
        });
    let running = items.iter().any(|item| {
        matches!(item.body, ItemBody::ToolCall { .. }) && item.status == ItemStatus::Running
    });
    let busy = items
        .iter()
        .any(|item| bingo_sdk::transcript::live_call(item, &rows.state.interactions).is_some())
        || rows
            .state
            .interactions
            .iter()
            .any(|ask| ask.turn.as_ref() == Some(&turn.id));
    let answering = items.iter().any(|item| {
        matches!(item.body, ItemBody::Assistant { .. }) && !rows.followed.contains(&item.id)
    });
    let stopping = rows.stop_asked == Some(&turn.id);
    if !stopping && (busy || (thought.is_none() && (answering || turn.retrying.is_some()))) {
        return (None, None);
    }
    if !stopping && thought.is_none() {
        let last = items
            .iter()
            .map(|item| {
                item.completed_at
                    .unwrap_or(item.started_at)
                    .max(item.started_at)
            })
            .max()
            .unwrap_or(turn.started_at);
        let due = last + jiff::SignedDuration::from_millis(500);
        if rows.now.wall < due {
            return (None, Some(rows.now.deadline(due)));
        }
    }
    (
        Some(Row {
            props: spinner_props(rows, items, thought.is_some(), running, stopping),
            thought,
        }),
        None,
    )
}

fn spinner_props(
    rows: &Rows<'_>,
    items: &[&Item],
    thinking: bool,
    running: bool,
    stopping: bool,
) -> Props {
    let word = if thinking {
        "thinking"
    } else {
        rows.slots
            .iter()
            .find_map(|placed| match &placed.slot {
                bingo_sdk::slot::Slot::Activity { verb, .. } => verb.as_deref(),
                _ => None,
            })
            .unwrap_or("working")
    };
    let responding = items.iter().any(|item| {
        matches!(item.body, ItemBody::Assistant { .. })
            && item.status == ItemStatus::Running
            && !rows.followed.contains(&item.id)
    });
    let mode = if running {
        "tool-use"
    } else if thinking {
        "thinking"
    } else if responding {
        "responding"
    } else {
        "requesting"
    };
    Props::Spinner(SpinnerProps {
        word: word.into(),
        message: stopping.then(|| "stopping".into()),
        suffix: String::new(),
        mode: mode.into(),
    })
}

pub fn props(rows: &Rows<'_>) -> Option<Props> {
    prepare(rows).0.map(|row| row.props)
}

pub fn draw(state: &bingo_sdk::SessionState, rows: &Rows<'_>) -> views::Drawing {
    let (row, next) = prepare(rows);
    let mut drawing = if let Some(row) = row {
        super::resolve(
            state,
            rows.pulled,
            "Spinner",
            &row.props,
            rows.width,
            &views::Marks::at(rows.now),
            |props, width| engine(props, row.thought, width, rows.now),
        )
    } else {
        views::Drawing {
            lines: Vec::new(),
            next,
        }
    };
    if let Some(retry) = state.turn.as_ref().and_then(|turn| turn.retrying) {
        drawing.lines.push(Line::from(Span::styled(
            format!("   retrying {}/{}", retry.attempt, retry.max),
            theme::ash(),
        )));
    }
    drawing
}

pub fn requests(rows: &Rows<'_>, frame: &mut crate::layout::Layout, height: u16) {
    let from = frame.blocks.closing_start();
    let to = from + draw(rows.state, rows).lines.len();
    if frame
        .visible_lines(usize::from(height))
        .iter()
        .any(|range| to > range.start && from < range.end)
        && let Some(props) = props(rows)
        && let Some(key) = super::key(rows.state, "Spinner", &props, rows.width as u16, height)
    {
        frame.pulls.push(key);
    }
}

fn engine(props: &Props, thought: Option<&Item>, width: usize, now: Now) -> views::Drawing {
    let Props::Spinner(props) = props else {
        return Default::default();
    };
    let glyph = node(theme::glyphs::get("still"), "glow", TextMotion::Spin);
    let word = node(
        &format!(
            "{}{}",
            props.message.as_deref().unwrap_or(&props.word),
            props.suffix
        ),
        "muted",
        TextMotion::Shimmer,
    );
    let view = View::Text {
        children: vec![
            TextChild::String("   ".into()),
            glyph,
            TextChild::String(" ".into()),
            word,
        ],
        style: Default::default(),
    };
    let mut drawn = views::marked(&view, width, &views::Marks::at(now));
    if let Some(thought) = thought {
        let elapsed = now
            .wall
            .duration_since(thought.started_at)
            .max(jiff::SignedDuration::ZERO);
        if elapsed.as_secs() >= 1
            && let Some(line) = drawn.lines.last_mut()
        {
            let clock = bingo_sdk::component::row::fmt_dur(elapsed.as_millis() as u64);
            *line =
                crate::layout::grid::pad_to(line.clone(), width.saturating_sub(2 + clock.width()));
            line.spans.push(Span::styled(clock, theme::dim()));
        }
        drawn.next = drawn
            .next
            .into_iter()
            .chain(bingo_sdk::transcript::next_second(now.wall, elapsed).map(|at| now.deadline(at)))
            .min();
    }
    drawn
}

fn node(text: &str, color: &str, motion: TextMotion) -> TextChild {
    TextChild::Node(Box::new(View::Text {
        children: vec![TextChild::String(text.into())],
        style: TextStyle {
            color: Some(color.into()),
            motion: Some(Box::new(motion)),
            ..Default::default()
        },
    }))
}

#[cfg(test)]
mod tests;
