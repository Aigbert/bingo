//! Arrival supplies the pace; the last characters carry the streaming ink.
use super::*;

pub fn closed(text: &str, status: ItemStatus) -> String {
    if !matches!(status, ItemStatus::Running | ItemStatus::Interrupted) {
        return text.to_owned();
    }
    // An open fence belongs to the markdown parser, not inline closing.
    if text
        .lines()
        .filter(|line| line.trim_start().starts_with("```"))
        .count()
        % 2
        == 1
    {
        return text.to_owned();
    }
    let mut text = text.to_owned();
    if text.ends_with('*') && !text.ends_with("**") {
        text.pop();
    }
    for mark in ["`", "**"] {
        if text.matches(mark).count() % 2 == 1 {
            if text.ends_with(mark) {
                text.truncate(text.len() - mark.len());
            } else {
                text.push_str(mark);
            }
            break;
        }
    }
    text
}

pub fn finish(
    lines: &mut Vec<Line<'static>>,
    status: ItemStatus,
    width: usize,
    now: Now,
    live: bool,
) {
    if status == ItemStatus::Running && !live {
        return;
    }
    let streaming = status == ItemStatus::Running;
    if streaming && now.motion {
        wet(lines);
    }
    let (suffix, ink, gap) = match status {
        ItemStatus::Running => (theme::glyphs::get("caret").to_owned(), theme::glow(), 0),
        ItemStatus::Interrupted => (
            format!("{} stopped", theme::glyphs::get("minus")),
            theme::dim(),
            1,
        ),
        _ => return,
    };
    if lines
        .last()
        .is_none_or(|line| line.width() + gap + suffix.width() > width)
    {
        lines.push(Line::default());
    }
    if let Some(line) = lines.last_mut() {
        if gap > 0 && line.width() > 0 {
            line.spans.push(Span::raw(" "));
        }
        line.spans.push(Span::styled(suffix, ink));
    }
}

fn wet(lines: &mut [Line<'static>]) {
    let mut left = 10;
    for line in lines.iter_mut().rev() {
        let mut spans = Vec::new();
        for span in line.spans.iter().rev() {
            for c in span.content.chars().rev() {
                let mut style = line.style.patch(span.style);
                if left > 0 {
                    let ink = theme::ink::foreground(style.fg, theme::palette());
                    style = style.patch(theme::literal(theme::ink::mix(
                        theme::palette().glow(),
                        ink,
                        (10 - left) as f64 / 10.0,
                    )));
                    left -= 1;
                }
                spans.push(Span::styled(c.to_string(), style));
            }
        }
        spans.reverse();
        line.spans = spans;
        if left == 0 {
            break;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn open_inline_marks_close_without_closing_a_code_fence() {
        assert_eq!(closed("a **bold", ItemStatus::Running), "a **bold**");
        assert_eq!(closed("a `code", ItemStatus::Running), "a `code`");
        assert_eq!(
            closed("```rust\nlet x = 1", ItemStatus::Running),
            "```rust\nlet x = 1"
        );
    }
    #[test]
    fn answer_ends_wrap_and_reduced_motion_keeps_the_ink() {
        let now = crate::test_support::scene().1;
        let mut rows = vec![Line::from(Span::styled("full", theme::text()))];
        finish(
            &mut rows,
            ItemStatus::Running,
            4,
            crate::test_support::still(now),
            true,
        );
        assert_eq!(rows[0].spans[0].style, theme::text());
        assert_eq!(rows[1].to_string(), theme::glyphs::get("caret"));
        let mut rows = vec![Line::from("full")];
        finish(&mut rows, ItemStatus::Interrupted, 10, now, false);
        assert_eq!(
            rows[1].to_string(),
            format!("{} stopped", theme::glyphs::get("minus"))
        );
    }
}
