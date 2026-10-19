//! A permission or a confirmation drawn in the dock: its title, what it
//! would do, then its answers as a pick list that fades in while the guard
//! holds keys back.
use super::{
    guard,
    spec::{Ask, Opt},
};
use crate::{
    clock::{self, Now},
    theme,
    views::{self, Drawing},
};
use bingo_sdk::pick;
use jiff::Timestamp;
use ratatui::text::Line;
use std::time::Instant;

/// Where the person is in an ask: the answer picked, the words typed into
/// it, when the ask opened and when a key last came.
pub struct Picking<'a> {
    pub cursor: usize,
    pub typing: Option<&'a str>,
    pub opened: Timestamp,
    pub keyed_at: Instant,
}

pub fn lines(ask: &Ask, picking: &Picking<'_>, width: u16, now: Now) -> Drawing {
    let mut out = vec![title(ask, width), Line::default()];
    out.extend(super::body::lines(&ask.body, usize::from(width)));
    out.push(Line::default());
    let fade = guard::fade(picking.opened, now);
    let (caret_now, rest) = rested(now, picking.keyed_at);
    for (index, option) in ask.options.iter().enumerate() {
        let picked = index == picking.cursor;
        let typing = picking
            .typing
            .filter(|_| picked && option.does.takes_words());
        let line = option_line(option, index, picked, typing, width);
        out.push(if picked {
            lit(line, fade, typing.is_some(), caret_now)
        } else {
            line
        });
    }
    let typing = picking.typing.is_some();
    let caret = (typing && caret_now.motion).then(|| (now.instant + clock::FRAME).min(rest));
    Drawing {
        lines: out,
        next: guard::next(picking.opened, now)
            .into_iter()
            .chain(caret)
            .min(),
    }
}

fn title(ask: &Ask, width: u16) -> Line<'static> {
    let ink = if ask.danger { "bad" } else { "fg" };
    let right = pick::line(ask.right.iter().map(|(text, ink)| pick::text(text, ink)));
    let row = pick::Row::new("title", pick::bold(&ask.title, ink)).right(right);
    drawn(
        &pick::pick_line(&row, false, width, theme::glyphs::set()),
        width,
    )
}

fn option_line(
    option: &Opt,
    index: usize,
    picked: bool,
    typing: Option<&str>,
    width: u16,
) -> Line<'static> {
    let left = match typing {
        Some(typed) => pick::line([
            pick::text(format!("{}: ", option.label), "muted"),
            pick::text(typed, "fg"),
            pick::text(theme::glyphs::get("caret"), "glow"),
        ]),
        None => pick::line(words(option, picked)),
    };
    let row = pick::Row::new(index.to_string(), left);
    drawn(
        &pick::pick_line(&row, picked, width, theme::glyphs::set()),
        width,
    )
}

/// An answer's words: its label, the scope a session answer allows, and
/// what a row that takes words says before any are typed.
fn words(option: &Opt, picked: bool) -> Vec<bingo_sdk::View> {
    let tone = option.tone.unwrap_or("fg");
    let said = |text: &str| {
        if picked {
            pick::bold(text, tone)
        } else {
            pick::text(text, "muted")
        }
    };
    let mut out = vec![said(&option.label)];
    if let Some(scope) = &option.scope {
        out.push(pick::text(" ", "fg"));
        out.push(pick::text(scope, if picked { "code" } else { "dim" }));
        out.push(said(option.tail));
    }
    if let Some(hint) = option.hint.as_ref().filter(|_| picked) {
        out.push(pick::text(format!("  {hint}"), "dim"));
    }
    out
}

fn drawn(view: &bingo_sdk::View, width: u16) -> Line<'static> {
    views::render(view, usize::from(width))
        .into_iter()
        .next()
        .unwrap_or_default()
}

/// The picked row rises from the dock's ground while the guard holds, and
/// its caret breathes while words are typed.
pub(super) fn lit(mut line: Line<'static>, fade: f64, typing: bool, now: Now) -> Line<'static> {
    let pick = theme::ground("pick").and_then(|style| style.bg);
    let quote = bingo_sdk::glyph::Glyph::Quote.spell(theme::glyphs::set());
    let caret = theme::glyphs::get("caret");
    if fade < 1.0 {
        let ground = theme::blended_ground("you", "pick", fade).and_then(|style| style.bg);
        let mark = theme::blended("you", "glow", fade).unwrap_or_default();
        for span in &mut line.spans {
            if span.style.bg == pick {
                span.style.bg = ground;
            }
            if span.content == quote {
                span.style = span.style.patch(mark);
            }
        }
    }
    if typing
        && let Some(span) = line
            .spans
            .iter_mut()
            .rev()
            .find(|span| span.content == caret)
    {
        span.style = span
            .style
            .patch(theme::caret_ink(now, "glow", "pick").unwrap_or_default());
    }
    line
}

/// The caret rests ten seconds after the last key, as the composer's does.
pub(super) fn rested(now: Now, keyed_at: Instant) -> (Now, Instant) {
    let rest = keyed_at + crate::composer::CARET_REST;
    (
        Now {
            motion: now.motion && now.instant < rest,
            ..now
        },
        rest,
    )
}
