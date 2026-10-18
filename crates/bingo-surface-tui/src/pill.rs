//! The way back to the foot of the transcript, drawn only while the foot is
//! off the screen.
//!
//! The last stream row carries the pill while the foot is off screen.
//! Its count is derived from the visible viewport; it owns no scroll state.

use ratatui::text::{Line, Span};
use unicode_width::UnicodeWidthStr;

use crate::theme;

/// The pill as a band row draws it: the line, and the cells it covers.
pub struct Pill {
    pub line: Line<'static>,
    pub x: u16,
    pub width: u16,
}

/// The count, centred on the input ground.
pub fn pill(below: usize, width: usize) -> Option<Pill> {
    let (lead, _) = label(below, width)?;
    let text = format!(" {lead} ");
    let used = text.width();
    let x = width.saturating_sub(used) / 2;
    let line = Line::from(vec![
        Span::raw(" ".repeat(x)),
        Span::styled(text, theme::text().patch(crate::dock::ground())),
    ]);
    Some(Pill {
        line,
        x: u16::try_from(x).unwrap_or(u16::MAX),
        width: u16::try_from(used).unwrap_or(u16::MAX),
    })
}

/// The words, as much of them as `width` holds with a cell of air each side:
/// the count and the hint, the count alone, the arrow alone — and nothing on
/// a row too narrow for even that. No count when there is nothing below: a
/// resize can hold a transcript whose foot is on the screen.
pub fn label(below: usize, width: usize) -> Option<(String, Option<&'static str>)> {
    let lead = match below {
        0 => theme::glyphs::get("down").to_string(),
        1 => format!("{} 1 line below", theme::glyphs::get("down")),
        many => format!("{} {many} lines below", theme::glyphs::get("down")),
    };
    let fits = |text: &str| text.width() + 2 <= width;
    if fits(&lead) {
        return Some((lead, None));
    }
    fits("↓").then(|| ("↓".to_string(), None))
}

#[cfg(test)]
pub fn below(ui: &crate::ui::Ui, now: crate::clock::Now) -> Option<usize> {
    if ui.scroll.following() {
        return None;
    }
    let (total, rows) = ui.transcript();
    Some(total.saturating_sub(ui.scroll.top(total, rows, now.instant) + rows))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn words(below: usize, width: usize) -> Option<String> {
        label(below, width).map(|(lead, hint)| match hint {
            Some(hint) => format!("{lead} · {hint}"),
            None => lead,
        })
    }

    #[test]
    fn the_label_says_how_far_the_foot_is_and_how_to_reach_it() {
        assert_eq!(words(37, 80).as_deref(), Some("↓ 37 lines below"));
        assert_eq!(words(1, 80).as_deref(), Some("↓ 1 line below"));
        assert_eq!(
            words(0, 80).as_deref(),
            Some("↓"),
            "held with the foot on the screen counts nothing"
        );
    }

    #[test]
    fn a_narrow_row_keeps_the_count_and_then_the_arrow() {
        assert_eq!(words(37, 30).as_deref(), Some("↓ 37 lines below"));
        assert_eq!(words(37, 10).as_deref(), Some("↓"));
        assert_eq!(words(37, 2), None, "no room for the air round it");
    }

    #[test]
    fn the_pill_sits_in_the_middle_of_the_row() {
        let pill = pill(37, 80).expect("a pill");
        let left = usize::from(pill.x);
        let right = 80 - usize::from(pill.x + pill.width);
        assert!(left.abs_diff(right) <= 1, "{left} left, {right} right");
        let text: String = pill.line.spans.iter().map(|s| s.content.as_ref()).collect();
        assert_eq!(
            text,
            format!("{}{}", " ".repeat(left), " ↓ 37 lines below ")
        );
    }
}
