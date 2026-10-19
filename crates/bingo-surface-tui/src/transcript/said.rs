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
/// room is the conversation: they are drawn as one ([`super::post`]).
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
) -> crate::views::Drawing {
    if let Some(headline) = item.headline() {
        return super::notice::draw(
            &headline,
            item.started_at.as_millisecond(),
            rows.width,
            rows.now,
        );
    }

    if rows.state.summary.driver == Driver::Log {
        return super::room::post(item, place, rows);
    }
    if is_post(origin) {
        return their_post(item, parts, origin, place, rows);
    }
    match voice(origin) {
        bingo_sdk::Voice::Aside => Default::default(),
        bingo_sdk::Voice::Quiet => notice(item, parts, origin, fold, rows),
        bingo_sdk::Voice::Said | bingo_sdk::Voice::Note => super::band::draw(item, rows),
    }
}

/// Whether an input is somebody's post rather than your words: its origin
/// names a principal — a teammate, a person on a channel — other than the
/// session that holds this one, whose words are what it was handed
/// ([`super::band`]).
pub fn is_post(origin: &Origin) -> bool {
    voice(origin) != bingo_sdk::Voice::Aside
        && origin
            .principal
            .as_deref()
            .is_some_and(|name| !bingo_collab::name::same(name, seats::HOLDER))
}

/// Who signed an input, where it is a post.
fn signed(item: &Item) -> Option<&str> {
    match &item.body {
        bingo_sdk::ItemBody::User { origin, .. } if is_post(origin) => origin.principal.as_deref(),
        _ => None,
    }
}

/// Whether an item's post follows one by the same voice, which it joins
/// without a blank row or a second name.
pub fn joins(item: &Item, previous: Option<&Item>, state: &bingo_sdk::SessionState) -> bool {
    let Some(previous) = previous else {
        return false;
    };
    if state.summary.driver == Driver::Log {
        return super::room::joins(item, Some(previous));
    }
    signed(item)
        .zip(signed(previous))
        .is_some_and(|(a, b)| bingo_collab::name::same(a, b))
}

/// A teammate's words outside a room: a report, a message.
fn their_post(
    item: &Item,
    parts: &[ContentPart],
    origin: &Origin,
    place: &super::Place<'_>,
    rows: &Rows<'_>,
) -> crate::views::Drawing {
    let Some(name) = origin.principal.as_deref() else {
        return Default::default();
    };
    let joined = joins(item, place.previous, rows.state);
    let voice = super::post::Voice {
        name,
        ink: rows.faces.who(name),
    };
    let at = item.started_at.as_millisecond();
    super::post::theirs(&voice, said(parts).trim(), at, joined, rows)
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
) -> crate::views::Drawing {
    let text = said(parts);
    let text = text.trim();
    if text.is_empty() {
        return Default::default();
    }
    let (head, rest) = text.split_once('\n').unwrap_or((text, ""));
    // A blank line between the headline and the rest is a separator in the
    // text — a command's prompt puts one there — and under a `⎿` it would be
    // a row spent on nothing.
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
    out.into()
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
