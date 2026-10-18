//! A background completion settles into a quiet divider.
use crate::{
    clock::Now,
    layout::grid,
    theme,
    views::{self, Drawing},
};
use bingo_sdk::view::TextMotion;
use ratatui::text::{Line, Span};

pub(super) fn draw(head: &str, at: i64, width: usize, now: Now) -> Drawing {
    let motion = TextMotion::Fade {
        from: "glow".into(),
        at_ms: at,
        ms: 400,
        curve: None,
    };
    let mut label = label(head);
    label.spans.insert(0, Span::styled(" ", theme::ash()));
    label.spans.push(Span::styled(" ", theme::ash()));
    for span in &mut label.spans {
        let ink = ["good", "bad", "code", "faint", "muted"]
            .into_iter()
            .find(|name| theme::foreground(name).is_some_and(|style| style.fg == span.style.fg))
            .and_then(|name| theme::palette().named(name))
            .unwrap_or(theme::palette().fg());
        span.style = views::motion::style(span.style, ink, Some(&motion), Some(now));
    }
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

fn label(head: &str) -> Line<'static> {
    let mark = head
        .strip_prefix("✓ ")
        .map(|text| (text, theme::glyphs::get("ok"), theme::good()))
        .or_else(|| {
            head.strip_prefix("× ")
                .map(|text| (text, theme::glyphs::get("times"), theme::bad()))
        });
    let (text, mark, ink) = mark.unwrap_or((head, "", theme::ash()));
    let mut line = Line::default();
    for (index, part) in text.split(" · ").enumerate() {
        if index > 0 {
            line.spans.push(Span::styled(
                theme::glyphs::get("sep"),
                theme::foreground("faint").unwrap_or_default(),
            ));
        }
        line.spans
            .extend(crate::markdown::inline(part, theme::ash()).spans);
    }
    if !mark.is_empty() {
        line.spans.insert(0, Span::styled(format!("{mark} "), ink));
    }
    line
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::{later, scene, still};

    #[test]
    fn a_notice_has_a_divider_and_its_arrival_keeps_a_deadline_without_room() {
        theme::with(crate::painted::truecolor(), || {
            let now = scene().1;
            let text = "✓ `cargo test` finished in the background · 2 passed";
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
