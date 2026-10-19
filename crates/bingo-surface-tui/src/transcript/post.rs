//! A post: someone's words under their name, read the way a chat is read
//! (`theirPostLines`, `yourPostLines`, `typingLines`). A name column, then the
//! words; consecutive posts by one voice share the name.
use super::Rows;
use crate::{clock::Now, layout::grid, markdown, theme, views::Drawing, wrap};
use bingo_sdk::view::TextMotion;
use ratatui::style::Style;
use ratatui::text::{Line, Span};

/// The name column's width: the words start at `TX + GUT`.
pub(crate) const GUT: usize = 11;
/// How long someone's words take to rise out of the ground.
const RISE_MS: u64 = 260;

/// Who a post is from, as the stream draws them.
pub struct Voice<'a> {
    pub name: &'a str,
    pub ink: &'static str,
}

/// Someone else's words: the quote in their ink, their name once, and the
/// words rising out of the ground as they arrive.
pub fn theirs(voice: &Voice<'_>, text: &str, at_ms: i64, joined: bool, rows: &Rows<'_>) -> Drawing {
    let rise = TextMotion::Fade {
        from: "bg".into(),
        at_ms,
        ms: RISE_MS,
        curve: None,
    };
    let ink = crate::face::style(voice.ink);
    let quote = Span::styled(theme::glyphs::get("quote"), ink);
    let lines = wrap::wrap_all(&prose(text), words_width(rows, 0))
        .into_iter()
        .enumerate()
        .map(|(row, words)| {
            let name = (row == 0 && !joined).then(|| (voice.name, ink.patch(theme::bold())));
            laid(Some(quote.clone()), name, risen(words, &rise, rows.now))
        })
        .collect();
    Drawing {
        lines,
        next: crate::views::motion::next(&rise, rows.now),
    }
}

/// Your words in a room, on your ground: each `@name` in its ink, and while
/// anyone named still owes an answer, who, with the edge breathing in the
/// first one's ink.
pub fn yours(text: &str, owed: &[String], joined: bool, rows: &Rows<'_>) -> Drawing {
    let right = (!owed.is_empty()).then(|| {
        Line::from(Span::styled(
            format!("waiting on {}", owed.join(", ")),
            theme::dim(),
        ))
    });
    let room = right.as_ref().map_or(0, |right| right.width() + 2);
    let (light, next) = match owed.first() {
        Some(first) => {
            let (style, next) = crate::face::breathing(rows.faces.who(first), rows.now);
            (Some(Span::styled(theme::glyphs::get("side"), style)), next)
        }
        None => (None, None),
    };
    let said: Vec<_> = text.split('\n').map(|line| called(line, rows)).collect();
    let name = theme::dim();
    let lines = wrap::wrap_all(&said, words_width(rows, room))
        .into_iter()
        .enumerate()
        .map(|(row, words)| {
            let who = (row == 0 && !joined).then_some(("you", name));
            let mut line = laid(light.clone(), who, words);
            if row == 0
                && let Some(right) = &right
            {
                line = grid::pad_to(line, rows.width.saturating_sub(grid::RM + right.width()));
                line.spans.extend(right.spans.iter().cloned());
            }
            grid::pad_to(line, rows.width).style(theme::ground("you").unwrap_or_default())
        })
        .collect();
    Drawing { lines, next }
}

/// Someone called on, writing their answer: the quote in their faded ink,
/// their name, and a spinner in their ink.
pub fn writing(voice: &Voice<'_>, now: Now) -> Drawing {
    let palette = theme::palette();
    let ink = crate::face::style(voice.ink);
    let faded = match (palette.named(voice.ink), palette.named("bg")) {
        (Some(ink), Some(bg)) => theme::literal(theme::ink::mix(ink, bg, 0.45)),
        _ => ink,
    };
    let words = Line::from(vec![
        Span::styled(crate::spinner::glyph(now), ink),
        Span::styled(" writing", theme::dim()),
    ]);
    let quote = Span::styled(theme::glyphs::get("quote"), faded);
    Drawing {
        lines: vec![laid(
            Some(quote),
            Some((voice.name, ink.patch(theme::bold()))),
            words,
        )],
        next: now.motion.then(|| crate::spinner::next(now)),
    }
}

/// What fits beside the name column, within the measure, less `room` on the
/// right.
fn words_width(rows: &Rows<'_>, room: usize) -> usize {
    rows.measured()
        .saturating_sub(grid::TX + GUT + grid::RM + room)
        .clamp(1, grid::MEASURE)
}

/// What someone wrote, line by line, in the text everything said is drawn in.
fn prose(text: &str) -> Vec<Line<'static>> {
    text.trim()
        .split('\n')
        .map(|line| markdown::inline(line, theme::text()))
        .collect()
}

/// One line of yours with each `@name` in its ink and `@all` in the room's.
fn called(line: &str, rows: &Rows<'_>) -> Line<'static> {
    let mut out = Line::default();
    let mut plain = 0;
    for (at, end) in calls(line) {
        out.spans
            .extend(markdown::inline(&line[plain..at], theme::text()).spans);
        let name = &line[at + 1..end];
        let ink = match name {
            "all" => "plan",
            name => rows.faces.who(name),
        };
        out.spans.push(Span::styled(
            line[at..end].to_owned(),
            crate::face::style(ink),
        ));
        plain = end;
    }
    out.spans
        .extend(markdown::inline(&line[plain..], theme::text()).spans);
    out
}

/// Where each `@name` in a line starts and ends: an `@` after a letter is an
/// address, not a call.
fn calls(line: &str) -> Vec<(usize, usize)> {
    let named = |c: char| c.is_alphanumeric() || matches!(c, '_' | '-');
    line.char_indices()
        .filter(|&(at, c)| c == '@' && !line[..at].chars().next_back().is_some_and(named))
        .filter_map(|(at, _)| {
            let end = line[at + 1..]
                .char_indices()
                .find(|&(_, c)| !named(c))
                .map_or(line.len(), |(end, _)| at + 1 + end);
            (end > at + 1).then_some((at, end))
        })
        .collect()
}

/// The words, each span rising from the ground into its own ink.
fn risen(words: Line<'static>, rise: &TextMotion, now: Now) -> Line<'static> {
    let spans = words
        .spans
        .into_iter()
        .map(|span| {
            let ink = span.style.fg.or(theme::text().fg).unwrap_or_default();
            let style = crate::views::motion::style(span.style, ink, Some(rise), Some(now));
            Span::styled(span.content, style)
        })
        .collect::<Vec<_>>();
    Line::from(spans)
}

/// A post's row on the grid: the edge at column 0, the name at `TX`, the
/// words at `TX + GUT`.
fn laid(
    edge: Option<Span<'static>>,
    name: Option<(&str, Style)>,
    words: Line<'static>,
) -> Line<'static> {
    let mut line = grid::pad_to(Line::from(edge.into_iter().collect::<Vec<_>>()), grid::TX);
    let name = name.map_or_else(Line::default, |(name, style)| {
        Line::from(Span::styled(name.to_owned(), style))
    });
    line.spans
        .extend(grid::pad_to(name, GUT.saturating_sub(1)).spans);
    line.spans.push(Span::raw(" "));
    line.spans.extend(words.spans);
    line
}
