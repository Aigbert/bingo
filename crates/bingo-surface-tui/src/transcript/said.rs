//! What was said into a session, as one transcript draws it: a person's own
//! line on its bar, a subsystem reporting in on a marked row, and — in a room's
//! own view — the conversation itself.
//!
//! One `ItemBody::User` is every one of those. Which it is comes off the origin
//! the kernel stamped on it and the session it is being read in, both read at
//! render time (ADR-0002): nothing here remembers a thing, and no kind of
//! saying is a second representation of another.

use bingo_sdk::{ContentPart, Driver, Item, Origin};
use ratatui::text::{Line, Span};
use unicode_width::UnicodeWidthStr;

use super::{OUTPUT_ROWS, Rows, bullet_style, kept, plain, returns, speaks, under};
use crate::fold::Fold;
use crate::skill;
use crate::{seats, theme};

/// Frozen compatibility for origins recorded before producers stamped voices.
const LEGACY_VOICES: &[(&str, bingo_sdk::Voice)] = &[
    ("agent", bingo_sdk::Voice::Quiet),
    ("bash", bingo_sdk::Voice::Quiet),
    ("command", bingo_sdk::Voice::Quiet),
    ("schedule", bingo_sdk::Voice::Quiet),
    ("room", bingo_sdk::Voice::Aside),
    ("wake", bingo_sdk::Voice::Note),
];

pub(crate) fn voice(origin: &Origin) -> bingo_sdk::Voice {
    origin.voice.unwrap_or_else(|| {
        LEGACY_VOICES.iter().find(|(surface, _)| *surface == origin.surface)
            .map(|(_, voice)| *voice).unwrap_or(bingo_sdk::Voice::Said)
    })
}

/// What a `User` item draws, which is a different thing in each of the two
/// places one is read.
///
/// In a room — a session nothing answers — every user item is a post, and the
/// room is the conversation: they are drawn as one ([`post`]).
///
/// Everywhere else, a room's activity is the room's: a member's transcript, the
/// holder's included, shows none of it, and a wake just opens a turn
/// (ADR-0034). The two things a room still puts in a member's journal are the
/// nudge and the reading its turn folded in; both are dropped here rather than
/// drawn, because the room's own view is where they are read and it is one
/// keystroke away. A peer's message still draws. Internal contributor context
/// has already been omitted at the item-block boundary.
pub fn lines(
    item: &Item,
    parts: &[ContentPart],
    origin: &Origin,
    fold: Fold,
    rows: &Rows<'_>,
) -> Vec<Line<'static>> {
    if rows.driver == Driver::Log {
        return post(item, parts, origin.principal.as_deref(), rows);
    }
    match voice(origin) {
        bingo_sdk::Voice::Aside => Vec::new(),
        bingo_sdk::Voice::Quiet => notice(item, parts, origin, fold, rows),
        bingo_sdk::Voice::Said | bingo_sdk::Voice::Note => user(parts, origin, rows),
    }
}

/// One post in the room's own transcript: who said it, then the whole of what
/// they said.
///
/// A room is read the way a chat is read, so the name leads and the message
/// follows it in the text everything said is drawn in — every line of it,
/// wrapped as prose. Nothing here hangs dim under a `⎿` and nothing folds
/// away: that is the shape of a subsystem reporting in ([`notice`]), and under
/// it the body of a message reads as collapsed metadata rather than as what
/// somebody wrote.
///
/// A post nobody signed came from the session the room hangs under — a person
/// at this composer, or the model that holds it — and every seat reads that one
/// under the roster's own word for it.
fn post(
    item: &Item,
    parts: &[ContentPart],
    principal: Option<&str>,
    rows: &Rows<'_>,
) -> Vec<Line<'static>> {
    let text = said(parts);
    let text = text.trim();
    if text.is_empty() {
        return Vec::new();
    }
    let mut body: Vec<Line<'static>> = text
        .lines()
        .map(|line| Line::from(Span::styled(line.to_string(), theme::text())))
        .collect();
    if let Some(first) = body.first_mut() {
        first
            .spans
            .insert(0, name(principal.unwrap_or(seats::HOLDER)));
    }
    speaks(bullet_style(item.status, false), body, rows)
}

/// Who said it: the one span a post spends on emphasis, so the eye finds the
/// author before it reads the line.
fn name(principal: &str) -> Span<'static> {
    Span::styled(format!("{principal}: "), theme::text().patch(theme::bold()))
}

/// What a `User` item says, as its parts spell it.
fn said(parts: &[ContentPart]) -> String {
    parts
        .iter()
        .filter_map(ContentPart::as_text)
        .collect::<Vec<_>>()
        .join("")
}

/// A subsystem's notice, marked the way a tool row is: the bullet, the one
/// line that says what happened, and the rest of it hanging under a `⎿` —
/// dim, folded, subordinate. The text already leads with the outcome, so the
/// first line is the summary and nothing has to be invented for it.
fn notice(
    item: &Item,
    parts: &[ContentPart],
    origin: &Origin,
    fold: Fold,
    rows: &Rows<'_>,
) -> Vec<Line<'static>> {
    let text = said(parts);
    let text = text.trim();
    if text.is_empty() {
        return Vec::new();
    }
    let (head, rest) = text.split_once('\n').unwrap_or((text, ""));
    // A blank line between the headline and the rest is a separator in the
    // text — a command's prompt puts one there — and under a `⎿` it would be
    // a row spent on nothing.
    let rest = rest.trim_start_matches('\n');
    // A skill is one row however it was asked for, so the typed line gives the
    // headline up to the run's own signature. The line itself stays in the
    // item, which is where a rewind reads it back.
    let style = bullet_style(item.status, false);
    let mut out = match skill::of(item, rows.commands) {
        // A person's own `/skill` line is not a call coming back, so its
        // words are at rest.
        Some(run) => super::skill_row(run, style, rows, super::Landing::Settled),
        None => speaks(
            style,
            vec![headline(
                head,
                origin.principal.as_deref(),
                elsewhere(origin, rows),
            )],
            rows,
        ),
    };
    if !rest.trim().is_empty() {
        out.extend(returns(kept(plain(rest), fold, OUTPUT_ROWS, None), rows));
    }
    out
}

/// The conversation a delivery says it came from, where saying it tells a
/// person something: a session is in several at once, and a delivery that names
/// the one already on the screen names nothing.
fn elsewhere<'a>(origin: &'a Origin, rows: &Rows<'_>) -> Option<&'a str> {
    origin
        .conversation
        .as_deref()
        .filter(|room| Some(*room) != rows.title)
}

/// The marked line itself: the sender's name where the origin carries one —
/// an agent, a room's member — where they said it, and what happened.
fn headline(head: &str, principal: Option<&str>, conversation: Option<&str>) -> Line<'static> {
    let mut spans = Vec::new();
    if let Some(name) = principal {
        let bold = theme::text().patch(theme::bold());
        spans.push(Span::styled(name.to_string(), bold));
        match conversation {
            // Who is bold, where is furniture: the room is dim so the name
            // still wins the row (design §2).
            Some(room) => spans.push(Span::styled(format!(" in {room}: "), theme::dim())),
            None => spans.push(Span::styled(": ".to_string(), bold)),
        }
    }
    spans.push(Span::styled(head.to_string(), theme::text()));
    Line::from(spans)
}

/// A person's own line, on a bar the width of the transcript. An origin that
/// names a principal is somebody else speaking — a channel's correspondent, a
/// person writing from elsewhere — so the line says who, as a chat does. Where
/// they said it is the view one is looking at; saying it again would be noise.
///
/// A wake is on the bar too and wears its own word instead of the `>`: it
/// opened this turn the way typing does, and it is not what anybody typed.
fn user(parts: &[ContentPart], origin: &Origin, rows: &Rows<'_>) -> Vec<Line<'static>> {
    let text = said(parts);
    if text.trim().is_empty() {
        return Vec::new();
    }
    let mut body: Vec<Line<'static>> = text
        .lines()
        .map(|line| Line::from(Span::styled(line.to_string(), theme::text())))
        .collect();
    if let Some(name) = origin.principal.as_deref()
        && let Some(first) = body.first_mut()
    {
        first.spans.insert(
            0,
            Span::styled(format!("{name}: "), theme::text().patch(theme::bold())),
        );
    }
    let mark = mark(origin);
    let indent = mark.content.width();
    barred(under(mark, body, indent, rows.measured()), rows.width)
}

/// Every row of a block on the raised bar. A shell line the person ran is on
/// the same bar for the same reason ([`super::ran`]): it is theirs.
pub(super) fn barred(lines: Vec<Line<'static>>, width: usize) -> Vec<Line<'static>> {
    lines.into_iter().map(|line| bar(line, width)).collect()
}

/// What stands in the gutter of a line on the bar: the theme's own `>` for
/// what a person typed, the word `wake` for the note a turn left itself. Its
/// width is the indent every line under it takes, so the two forms wrap alike.
fn mark(origin: &Origin) -> Span<'static> {
    let word = match voice(origin) == bingo_sdk::Voice::Note {
        true => &origin.surface,
        false => theme::user(),
    };
    Span::styled(format!("{word} "), theme::dim())
}

/// The raised bar behind a `>` line: it runs to the edge of the transcript,
/// so what you said is a band and not a sentence.
fn bar(line: Line<'static>, width: usize) -> Line<'static> {
    let mut spans = line.spans;
    let used: usize = spans.iter().map(|s| s.content.width()).sum();
    spans.push(Span::raw(" ".repeat(width.saturating_sub(used))));
    let mut line = Line::from(spans);
    line.style = theme::raised();
    line
}

#[cfg(test)]
mod voice_contract {
    use super::*;
    use bingo_sdk::Voice;

    #[test]
    fn stamped_voice_wins_and_legacy_words_are_frozen() {
        for (surface, legacy) in [("agent", Voice::Quiet), ("bash", Voice::Quiet),
            ("command", Voice::Quiet), ("schedule", Voice::Quiet),
            ("room", Voice::Aside), ("wake", Voice::Note), ("stranger", Voice::Said)] {
            let origin = Origin::surface(surface);
            assert_eq!(voice(&origin), legacy);
            for stamped in [Voice::Said, Voice::Note, Voice::Quiet, Voice::Aside] {
                assert_eq!(voice(&origin.clone().voiced(stamped)), stamped);
            }
        }
    }
}
