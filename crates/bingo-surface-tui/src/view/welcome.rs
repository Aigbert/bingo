//! The Welcome site shares the frame's transcript projection.
use crate::{layout::Layout, transcript::Rows};
use ratatui::{layout::Rect, widgets::Paragraph};
pub(super) fn render(rows: &Rows<'_>, frame: &mut Layout, area: Rect) {
    let out = rows
        .state
        .transcript
        .items()
        .iter()
        .find_map(|item| bingo_sdk::asked(item).map(|_| item.started_at));
    if (out.is_none() && !rows.state.transcript.items().is_empty())
        || out.is_some_and(|at| {
            !rows.now.motion || rows.now.wall.duration_since(at).as_millis() >= 520
        })
    {
        return;
    }
    if let Some(key) = crate::component::welcome::request(rows) {
        frame.pulls.push(key);
    }
    let own = crate::welcome::is_engine(rows.pulled);
    if own && let Some(index) = rows.entrance.selected {
        let x = ((area.width.saturating_sub(52)) / 2).max(3);
        let pick = Rect::new(
            area.x + x - 2,
            area.y + top_of(area.height) + 9 + index as u16,
            56.min(area.width.saturating_sub(x - 2)),
            1,
        );
        frame
            .canvas
            .buffer
            .set_style(pick, crate::theme::ground("pickBg").unwrap_or_default());
    }
    let drawing = crate::component::welcome::draw(rows);
    let lines = frame.collect(drawing);
    let top = top_of(area.height);
    let y = area.y.saturating_add(top);
    frame.render_widget(
        Paragraph::new(lines),
        Rect {
            y,
            height: area.height.saturating_sub(top),
            ..area
        },
    );
    if own && let Some(at) = rows.entrance.started_at_ms {
        crate::welcome::effects(
            &mut frame.canvas.buffer,
            area,
            top,
            at,
            out.map(|at| at.as_millisecond()),
            rows.now,
        );
    }
}

fn top_of(height: u16) -> u16 {
    ((usize::from(height).saturating_sub(14)) as f64 * 0.42)
        .floor()
        .max(1.0) as u16
}
