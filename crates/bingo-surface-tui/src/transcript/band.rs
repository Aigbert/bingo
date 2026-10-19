//! The prompt band reads its light, clock and additions from the turn.
use super::*;
use crate::layout::grid;
use bingo_sdk::{
    TurnStatus,
    component::{Props, TurnDurationProps},
};

pub fn owns(item: &Item, state: &SessionState) -> bool {
    state.summary.driver != bingo_sdk::Driver::Log && prompt(item)
}

fn prompt(item: &Item) -> bool {
    let ItemBody::User { origin, .. } = &item.body else {
        return false;
    };
    let voice = said::voice(origin);
    let correspondent = origin.principal.is_some() && voice != bingo_sdk::Voice::Aside;
    correspondent || matches!(voice, bingo_sdk::Voice::Said | bingo_sdk::Voice::Note)
}

pub fn first(item: &Item, rows: &Rows<'_>) -> bool {
    rows.turn_items(item)
        .iter()
        .find(|other| other.turn == item.turn && owns(other, rows.state))
        .is_none_or(|first| first.id == item.id)
}

pub fn draw(item: &Item, rows: &Rows<'_>) -> crate::views::Drawing {
    let first = item.turn.is_some() && first(item, rows);
    if item.turn.is_some() && !first && !named(item) {
        return Default::default();
    }
    let turn = Band::of(item, rows);
    let clock = if first {
        duration(item, &turn, rows)
    } else {
        Default::default()
    };
    let right = clock.lines.first().cloned().unwrap_or_default();
    let width = rows.width.saturating_sub(grid::TX + grid::RM).max(1);
    let prompt = text(item);
    let mut body = prompt_lines(item, &prompt);
    if let Some(line) = body.first_mut() {
        super::chips::prepend(item, line);
    }
    let mut lines = wrap::wrap_all(&body, width);
    let room = if first && (turn.live || right.width() > 0) {
        right.width().max(8) + 2
    } else {
        0
    };
    if room > 0
        && lines
            .first()
            .is_some_and(|line| line.width() > width.saturating_sub(room))
    {
        lines = wrap::wrap_all(&body, width.saturating_sub(room).max(1));
    }
    if lines.len() > 4 {
        let more = lines.len() - 3;
        lines.truncate(3);
        lines.push(Line::from(Span::styled(
            format!("{} {more} more lines", theme::glyphs::get("ell")),
            theme::dim(),
        )));
    }
    let mut out = crate::views::Drawing {
        lines: Vec::new(),
        next: clock.next,
    };
    for (at, line) in lines.into_iter().enumerate() {
        out.lines.push(laid(
            line,
            if at == 0 {
                right.clone()
            } else {
                Line::default()
            },
            rows.width,
        ));
    }
    if first {
        if !rows.opened.in_detail(item) {
            additions(item, rows, &mut out);
        }
        light(&turn, rows, &mut out);
    }
    out
}

fn text(item: &Item) -> String {
    match &item.body {
        ItemBody::User { parts, .. } => super::chips::text(item, &super::said::said(parts)),
        _ => String::new(),
    }
}

fn note(item: &Item) -> bool {
    matches!(&item.body, ItemBody::User { origin, .. }
        if origin.principal.is_none() && said::voice(origin) == bingo_sdk::Voice::Note)
}

fn prompt_lines(item: &Item, text: &str) -> Vec<Line<'static>> {
    let note = note(item);
    let ink = if note { theme::ash() } else { theme::text() };
    let mut lines: Vec<_> = text
        .split('\n')
        .map(|line| markdown::inline(line, ink))
        .collect();
    if let ItemBody::User { origin, .. } = &item.body {
        let label = if note {
            Some("wake-up")
        } else {
            origin.principal.as_deref()
        };
        if let Some(label) = label {
            lines[0]
                .spans
                .insert(0, Span::styled(format!("{label}  "), theme::glow()));
        }
    }
    lines
}

pub struct Band {
    live: bool,
    asking: bool,
    newest: bool,
    stopped: bool,
    failed: bool,
    end: jiff::Timestamp,
    clock: Option<bingo_sdk::transcript::TurnClock>,
}

impl Band {
    fn of(item: &Item, rows: &Rows<'_>) -> Self {
        let state = rows.state;
        let now = rows.now;
        let live = state
            .turn
            .as_ref()
            .filter(|turn| Some(&turn.id) == item.turn.as_ref());
        let last = state
            .last_turn
            .as_ref()
            .filter(|turn| Some(&turn.id) == item.turn.as_ref());
        let items = rows.turn_items(item);
        let end = last.map(|turn| turn.ended_at).unwrap_or(item.started_at);
        let clock = item.turn.as_ref().and_then(|id| {
            bingo_sdk::transcript::clock_of(
                items,
                bingo_sdk::transcript::Ends::of(state, id),
                now.wall,
            )
        });
        let stopped = last.is_some_and(|t| matches!(t.status, TurnStatus::Interrupted { .. }))
            || items
                .iter()
                .any(|i| matches!(i.body, ItemBody::Interruption { .. }));
        Self {
            live: live.is_some(),
            asking: live.is_some() && state.interactions.iter().any(|ask| ask.turn == item.turn),
            newest: rows.newest == item.turn.as_ref() && super::ran::latest(state).is_none(),
            stopped,
            failed: last.is_some_and(|t| matches!(t.status, TurnStatus::Failed { .. })),
            end,
            clock,
        }
    }

    fn elapsed(&self) -> u64 {
        self.clock
            .map_or(0, |clock| clock.elapsed.as_millis().max(0) as u64)
    }

    fn props(&self) -> Option<Props> {
        (self.elapsed() >= 1000 || self.stopped).then(|| {
            Props::TurnDuration(TurnDurationProps {
                word: if self.stopped { "stopped" } else { "" }.into(),
                duration_ms: self.elapsed() / 1000 * 1000,
            })
        })
    }
}

pub fn request(item: &Item, rows: &Rows<'_>) -> Option<(String, Props)> {
    if item.turn.is_none() || !owns(item, rows.state) || !first(item, rows) {
        return None;
    }
    Some((
        format!("{}:duration", item.id),
        Band::of(item, rows).props()?,
    ))
}

fn duration(item: &Item, turn: &Band, rows: &Rows<'_>) -> crate::views::Drawing {
    let mut drawn = if let Some(props) = turn.props() {
        crate::component::resolve(
            rows.state,
            rows.pulled,
            &format!("{}:duration", item.id),
            &props,
            rows.width,
            &crate::views::Marks::at(rows.now),
            |props, _| {
                let Props::TurnDuration(props) = props else {
                    return Vec::new();
                };
                let mut spans = Vec::new();
                if !props.word.is_empty() {
                    spans.push(Span::styled(props.word.clone(), theme::ash()));
                }
                if props.duration_ms >= 1000 {
                    if !spans.is_empty() {
                        spans.push(Span::styled(
                            theme::glyphs::get("sep"),
                            theme::foreground("faint").unwrap_or_default(),
                        ));
                    }
                    spans.push(Span::styled(
                        bingo_sdk::component::row::fmt_dur(props.duration_ms),
                        if turn.live {
                            theme::ash()
                        } else {
                            theme::dim()
                        },
                    ));
                }
                vec![Line::from(spans)]
            },
        )
    } else {
        Default::default()
    };
    drawn.next = drawn
        .next
        .into_iter()
        .chain(
            turn.clock
                .and_then(|clock| clock.next_second)
                .map(|at| rows.now.deadline(at)),
        )
        .min();
    drawn
}

fn laid(left: Line<'static>, right: Line<'static>, width: usize) -> Line<'static> {
    let ground = theme::ground("you").unwrap_or_default();
    let mut line = Line::from("   ");
    line.spans.extend(left.spans);
    if right.width() > 0 {
        line = grid::pad_to(line, width.saturating_sub(grid::RM + right.width()));
        line.spans.extend(right.spans);
    }
    grid::pad_to(line, width).style(ground)
}

fn additions(item: &Item, rows: &Rows<'_>, out: &mut crate::views::Drawing) {
    for other in rows
        .turns
        .get(&item.turn)
        .into_iter()
        .flatten()
        .filter(|other| other.id != item.id)
    {
        out.append(addition(other, rows));
    }
}

pub(super) fn addition(item: &Item, rows: &Rows<'_>) -> crate::views::Drawing {
    let (words, note) = match &item.body {
        ItemBody::User { .. } if owns(item, rows.state) && !named(item) => (text(item), "steered"),
        ItemBody::PermissionReceipt {
            feedback: Some(text),
            decision: bingo_sdk::DecisionKind::Deny,
            ..
        } => (text.clone(), "said no"),
        _ => return Default::default(),
    };
    let view = bingo_sdk::View::Text {
        children: vec![bingo_sdk::view::TextChild::String(note.into())],
        style: bingo_sdk::view::TextStyle {
            color: Some("dim".into()),
            motion: Some(Box::new(bingo_sdk::view::TextMotion::Fade {
                from: "glow".into(),
                at_ms: item.started_at.as_millisecond(),
                ms: 420,
                curve: None,
            })),
            ..Default::default()
        },
    };
    let note = crate::views::marked(&view, rows.width, &crate::views::Marks::at(rows.now));
    let right = note.lines.first().cloned().unwrap_or_default();
    let next = note.next;
    let room = rows
        .width
        .saturating_sub(grid::TX + grid::RM + right.width() + 2);
    let mut left = markdown::inline(words.lines().next().unwrap_or_default(), theme::text());
    super::chips::prepend(item, &mut left);
    crate::views::Drawing {
        lines: vec![laid(grid::fit(left, room), right, rows.width)],
        next,
    }
}

fn light(turn: &Band, rows: &Rows<'_>, out: &mut crate::views::Drawing) {
    use bingo_sdk::{
        View,
        view::{TextChild, TextMotion, TextStyle},
    };
    if !turn.live && !turn.newest {
        return;
    }
    let (ink, motion) = if turn.asking {
        ("amberGlow", None)
    } else if turn.live {
        (
            "glow",
            Some(TextMotion::Breathe {
                with: "coal".into(),
                period_ms: 1800,
                depth: 1.0,
            }),
        )
    } else if turn.stopped {
        ("dim", None)
    } else if turn.failed {
        ("bad", None)
    } else {
        (
            "good",
            Some(TextMotion::Fade {
                curve: Some(bingo_sdk::view::FadeCurve::EaseOut),
                from: "glow".into(),
                at_ms: turn.end.as_millisecond(),
                ms: 320,
            }),
        )
    };
    let view = View::Text {
        children: vec![TextChild::String(theme::glyphs::get("side").into())],
        style: TextStyle {
            color: Some(ink.into()),
            motion: motion.map(Box::new),
            ..Default::default()
        },
    };
    let drawn = crate::views::marked(&view, 1, &crate::views::Marks::at(rows.now));
    for line in &mut out.lines {
        if let Some(span) = line.spans.first_mut() {
            span.content = "  ".into();
        }
        if let Some(mark) = drawn.lines.first() {
            line.spans.splice(0..0, mark.spans.iter().cloned());
        }
    }
    out.next = out.next.into_iter().chain(drawn.next).min();
}

fn named(item: &Item) -> bool {
    matches!(&item.body, ItemBody::User { origin, .. } if origin.principal.is_some())
}

/// Only facts the band paints revise its memo; unrelated events do not.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Revision {
    live: bool,
    asking: bool,
    stopped: bool,
    failed: bool,
    seconds: u64,
    newest: bool,
    sparks: bool,
    detail: bool,
}

pub fn revision(item: &Item, rows: &Rows<'_>) -> Option<Revision> {
    if item.turn.is_none() || !owns(item, rows.state) || !first(item, rows) {
        return None;
    }
    let band = Band::of(item, rows);
    Some(Revision {
        detail: rows.opened.in_detail(item),
        live: band.live,
        asking: band.asking,
        stopped: band.stopped,
        failed: band.failed,
        seconds: band.elapsed() / 1000,
        newest: band.newest,
        sparks: rows
            .state
            .last_turn
            .as_ref()
            .is_some_and(|turn| Some(&turn.id) == item.turn.as_ref())
            && !band.live
            && band.newest
            && !band.stopped
            && !band.failed
            && (0..700).contains(&rows.now.wall.duration_since(band.end).as_millis()),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::{folded, frame, scene, started, still, user};

    #[test]
    fn plain_prompt_and_steered_code_keep_backticks_and_their_own_ink() {
        theme::with(crate::painted::no_colour(), || {
            let prompt = user("prompt", "Run `cargo test` first");
            let state = folded(vec![
                frame(1, started("trn_1")),
                frame(
                    2,
                    bingo_sdk::Event::ItemCompleted {
                        item: prompt.clone(),
                    },
                ),
                frame(
                    3,
                    bingo_sdk::Event::ItemCompleted {
                        item: user("steer", "Try `cargo test` again"),
                    },
                ),
            ]);
            let folds = Default::default();
            let pictures = Default::default();
            let linked = Default::default();
            let rows = Rows::of(&state, 80, &folds, &pictures, &linked, still(scene().1));
            let drawn = draw(&prompt, &rows);
            for (line, words) in drawn
                .lines
                .iter()
                .zip(["Run `cargo test` first", "Try `cargo test` again"])
            {
                assert!(line.to_string().contains(words), "{line}");
                let start = line.to_string().find("`cargo test`").unwrap();
                let inks: Vec<_> = line
                    .spans
                    .iter()
                    .flat_map(|span| span.content.bytes().map(|_| span.style))
                    .skip(start)
                    .take("`cargo test`".len())
                    .collect();
                assert_eq!(inks, vec![theme::text(); "`cargo test`".len()]);
            }
            assert_eq!(drawn.lines.len(), 2);
            assert!(drawn.lines[1].to_string().ends_with("steered  "));
        });
    }
}
