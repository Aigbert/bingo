//! A background completion settles into a quiet divider.
use crate::{
    clock::Now,
    layout::grid,
    theme,
    views::{self, Drawing},
};
use bingo_sdk::view::TextMotion;
use ratatui::text::{Line, Span};

pub(super) fn draw(head: &bingo_sdk::Headline, at: i64, width: usize, now: Now) -> Drawing {
    let motion = TextMotion::Fade {
        from: "glow".into(),
        at_ms: at,
        ms: 400,
        curve: None,
    };
    let parts: Vec<_> = head.parts.iter().map(|part| part.as_str().into()).collect();
    let label = Line::from(crate::tone::parts(
        &parts,
        head.tone,
        crate::tone::Content::Markdown,
        Some((motion.clone(), now)),
    ));
    divider(label, motion, width, now)
}

fn divider(mut label: Line<'static>, motion: TextMotion, width: usize, now: Now) -> Drawing {
    let muted = theme::foreground("muted").unwrap_or_default();
    label.spans.insert(0, Span::styled(" ", muted));
    label.spans.push(Span::styled(" ", muted));
    let faint = theme::foreground("faint").unwrap_or_default();
    let thin = theme::glyphs::get("thin");
    let mut line = Line::from(vec![
        Span::raw(" ".repeat(grid::TX)),
        Span::styled(thin.repeat(2), faint),
    ]);
    line.spans.extend(label.spans);
    let rest = width.saturating_sub(grid::RM + line.width());
    line.spans.push(Span::styled(thin.repeat(rest), faint));
    Drawing {
        lines: vec![line],
        next: views::motion::next(&motion, now),
    }
}

pub(super) fn compacted(
    item: &bingo_sdk::Item,
    (before, after): (u64, u64),
    window: Option<u64>,
    rows: &super::Rows<'_>,
) -> Drawing {
    let numbers = if let Some(window) = window {
        let percent = |used: u64| (used as f64 / window.max(1) as f64 * 100.0).round();
        format!(
            "context {}% {} {}%",
            percent(before),
            theme::glyphs::get("arrow"),
            percent(after)
        )
    } else {
        format!(
            "context {before} {} {after} tokens",
            theme::glyphs::get("arrow")
        )
    };
    let motion = TextMotion::Fade {
        from: "glow".into(),
        at_ms: item.started_at.as_millisecond(),
        ms: 400,
        curve: None,
    };
    let label = Line::from(
        [
            ("compacted".to_owned(), "muted"),
            (theme::glyphs::get("sep").into(), "faint"),
            (numbers, "dim"),
        ]
        .into_iter()
        .map(|(text, ink)| {
            let style = theme::foreground(ink).unwrap_or_default();
            Span::styled(
                text,
                views::motion::style(
                    style,
                    theme::palette().named(ink).unwrap_or(theme::palette().fg()),
                    Some(&motion),
                    Some(rows.now),
                ),
            )
        })
        .collect::<Vec<_>>(),
    );
    divider(label, motion, rows.width, rows.now)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::{later, scene, still};

    #[test]
    fn a_notice_has_a_divider_and_its_arrival_keeps_a_deadline_without_room() {
        theme::with(crate::painted::truecolor(), || {
            let now = scene().1;
            let text = &bingo_sdk::Headline {
                tone: bingo_sdk::Tone::Good,
                parts: vec![
                    "`cargo test` finished in the background".into(),
                    "2 passed".into(),
                ],
            };
            let settled = draw(text, now.wall.as_millisecond(), 80, later(now, 400));
            assert!(
                settled.lines[0]
                    .to_string()
                    .starts_with("   ── ✓ cargo test")
            );
            assert!(settled.lines[0].to_string().ends_with('─'));
            assert!(settled.next.is_none());
            let fresh = draw(text, now.wall.as_millisecond(), 80, now);
            assert!(fresh.next.is_some());
            assert_ne!(fresh.lines, settled.lines);
            assert_eq!(
                draw(text, now.wall.as_millisecond(), 0, now).next,
                fresh.next
            );
            assert!(
                draw(text, now.wall.as_millisecond(), 80, still(now))
                    .next
                    .is_none()
            );
        });
    }
}
