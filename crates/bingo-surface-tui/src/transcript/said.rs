//! What was said into a session, as one transcript draws it: a person's own
//! band with a name for correspondents, a subsystem notice, and — in a room's
//! own view — the conversation itself.
//!
//! One `ItemBody::User` is every one of those. Which it is comes off the origin
//! the kernel stamped on it and the session it is being read in, both read at
//! render time (ADR-0002): nothing here remembers a thing, and no kind of
//! saying is a second representation of another.

use bingo_sdk::{ContentPart, Driver, Item, Origin};
use ratatui::text::{Line, Span};

use super::{OUTPUT_ROWS, Rows, bullet_style, kept, plain, returns, speaks};
use crate::fold::Fold;
use crate::{seats, theme};

pub(crate) use bingo_sdk::effective_voice as voice;

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
    place: &super::Place<'_>,
    fold: Fold,
    rows: &Rows<'_>,
) -> Vec<Line<'static>> {
    if rows.state.summary.driver == Driver::Log {
        return post(item, place, rows);
    }
    match voice(origin) {
        bingo_sdk::Voice::Aside => Vec::new(),
        bingo_sdk::Voice::Quiet => notice(item, parts, origin, fold, rows),
        bingo_sdk::Voice::Said | bingo_sdk::Voice::Note => super::band::draw(item, rows).lines,
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
fn post(item: &Item, place: &super::Place<'_>, rows: &Rows<'_>) -> Vec<Line<'static>> {
    let Some(post) = bingo_collab::mentions::Post::of(item) else {
        return Vec::new();
    };
    if post.text.trim().is_empty() {
        return Vec::new();
    }
    let previous = place.previous.and_then(bingo_collab::mentions::Post::of);
    let grouped = previous
        .as_ref()
        .is_some_and(|before| bingo_collab::name::same(&before.author, &post.author));
    let mut body: Vec<Line<'static>> = post
        .text
        .trim()
        .lines()
        .map(|line| {
            Line::from(crate::markdown::mentions::spans(
                line,
                theme::text(),
                &rows.names,
            ))
        })
        .collect();
    if !grouped && let Some(first) = body.first_mut() {
        first.spans.insert(
            0,
            Span::styled(
                format!("{}: ", post.author),
                crate::face::style(&post.author).patch(theme::bold()),
            ),
        );
    }
    let members = seats::members(rows.state);
    if addressed(&post, &members)
        && let Some(last) = body.last_mut()
    {
        let arrow = if theme::glyphs() == &theme::ASCII {
            " -> you"
        } else {
            " → you"
        };
        last.spans.push(Span::styled(arrow, theme::dim()));
    }
    let glyph = if grouped {
        " "
    } else {
        crate::face::face(&post.author).sigil()
    };
    let mut out = super::marked(glyph, crate::face::style(&post.author), body, rows);
    if rows.first_post == Some(&item.id) {
        out.extend(member_row(&members, rows));
    }
    out
}

fn member_row(members: &[String], rows: &Rows<'_>) -> Vec<Line<'static>> {
    if members.is_empty() {
        return Vec::new();
    }
    super::under(
        Span::raw("  "),
        vec![crate::layout::grid::fit(
            Line::from(vec![Span::styled(
                format!("members: {}", members.join(", ")),
                theme::dim(),
            )]),
            rows.measured().saturating_sub(2),
        )],
        2,
        rows.measured(),
    )
}

/// The room's composer speaks as the holder; the shared matcher owns @all.
fn addressed(post: &bingo_collab::mentions::Post, members: &[String]) -> bool {
    if bingo_collab::name::same(&post.author, seats::HOLDER)
        || !members
            .iter()
            .any(|name| bingo_collab::name::same(name, seats::HOLDER))
    {
        return false;
    }
    bingo_collab::mentions::named(&post.text, members)
        .iter()
        .any(|owed| match owed {
            bingo_collab::mentions::Owed::Member(name) => {
                bingo_collab::name::same(name, seats::HOLDER)
            }
            bingo_collab::mentions::Owed::Room => true,
        })
}

/// What a `User` item says, as its parts spell it.
pub(super) fn said(parts: &[ContentPart]) -> String {
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
    if head.starts_with(['✓', '×']) {
        return vec![crate::views::indent(
            quiet_head(head),
            crate::layout::grid::TX,
        )];
    }
    let rest = rest.trim_start_matches('\n');
    let mut out = speaks(
        bullet_style(item.status, false),
        vec![headline(
            head,
            origin.principal.as_deref(),
            elsewhere(origin, rows),
        )],
        rows,
    );
    if !rest.trim().is_empty() {
        out.extend(returns(kept(plain(rest), fold, OUTPUT_ROWS, None), rows));
    }
    out
}

fn quiet_head(head: &str) -> Line<'static> {
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

/// The conversation a delivery says it came from, where saying it tells a
/// person something: a session is in several at once, and a delivery that names
/// the one already on the screen names nothing.
fn elsewhere<'a>(origin: &'a Origin, rows: &Rows<'_>) -> Option<&'a str> {
    origin
        .conversation
        .as_deref()
        .filter(|room| Some(*room) != rows.state.summary.title.as_deref())
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
/// Every row of a block on the raised bar. A shell line the person ran is on
/// the same bar for the same reason ([`super::ran`]): it is theirs.
#[cfg(test)]
mod voice_contract {
    use super::*;
    use bingo_sdk::Voice;

    #[test]
    fn stamped_voice_wins_and_unstamped_fails_closed() {
        let quiet = ["agent", "bash", "command", "room", "wake", "stranger"];
        let unstamped = quiet
            .map(|surface| (surface, Voice::Quiet))
            .into_iter()
            .chain(bingo_sdk::PERSON_SURFACES.map(|surface| (surface, Voice::Said)));
        for (surface, legacy) in unstamped {
            let origin = Origin::surface(surface);
            assert_eq!(voice(&origin), legacy);
            for stamped in [Voice::Said, Voice::Note, Voice::Quiet, Voice::Aside] {
                assert_eq!(voice(&origin.clone().voiced(stamped)), stamped);
            }
        }
    }
}
