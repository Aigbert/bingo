//! The session queue supplies every row; only its brief arrival motion has a clock.
use crate::{
    clock::{self, Now},
    layout::grid,
    theme, transcript,
    views::Drawing,
};
use bingo_sdk::{IntentId, QueueEntry, SessionState};
use ratatui::text::{Line, Span};
use std::{
    collections::BTreeMap,
    time::{Duration, Instant},
};

const COOL: Duration = Duration::from_millis(260);

#[derive(Debug, Default)]
pub struct Arrivals(BTreeMap<IntentId, Instant>);

impl Arrivals {
    pub fn observe(&mut self, before: &[QueueEntry], after: &[QueueEntry], now: Instant) {
        self.0
            .retain(|_, at| now.saturating_duration_since(*at) < COOL);
        for entry in after
            .iter()
            .filter(|entry| !before.iter().any(|old| old.intent == entry.intent))
        {
            self.0.insert(entry.intent.clone(), now);
        }
    }

    fn ink(&self, entry: &QueueEntry, now: Now) -> (ratatui::style::Style, Option<Instant>) {
        let muted = theme::foreground("muted").unwrap_or_default();
        let Some(at) = self.0.get(&entry.intent).filter(|_| now.motion) else {
            return (muted, None);
        };
        let age = now.since(*at);
        if age >= COOL {
            return (muted, None);
        }
        let color = theme::ink::mix(
            theme::palette().named("glow").unwrap_or_default(),
            theme::palette().named("muted").unwrap_or_default(),
            age.as_secs_f64() / COOL.as_secs_f64(),
        );
        (
            theme::literal(color),
            Some((now.instant + clock::FRAME).min(*at + COOL)),
        )
    }
}

pub fn draw(state: &SessionState, arrivals: &Arrivals, width: usize, now: Now) -> Drawing {
    let entries: Vec<_> = state
        .queue
        .iter()
        .filter(|entry| {
            matches!(
                transcript::voice(&entry.origin),
                bingo_sdk::Voice::Said | bingo_sdk::Voice::Note
            )
        })
        .collect();
    let waits = entries.iter().filter(|entry| !entry.steerable).count();
    let mut index = 0;
    let mut out = Drawing::default();
    for entry in entries {
        let right = if entry.steerable {
            "at the next step".into()
        } else {
            index += 1;
            if waits > 1 {
                format!("queued {index}")
            } else {
                "queued".into()
            }
        };
        let (ink, next) = arrivals.ink(entry, now);
        out.next = out.next.into_iter().chain(next).min();
        out.lines.push(row(&entry.preview, &right, ink, width));
    }
    out
}

fn row(text: &str, right: &str, ink: ratatui::style::Style, width: usize) -> Line<'static> {
    let right = Line::from(Span::styled(right.to_owned(), theme::dim()));
    let room = width.saturating_sub(grid::TX + grid::RM + right.width() + 4);
    let mut line = Line::from(" ".repeat(grid::TX));
    line.spans
        .extend(grid::fit(Line::from(Span::styled(text.replace('\n', " "), ink)), room).spans);
    line = grid::pad_to(line, width.saturating_sub(grid::RM + right.width()));
    line.spans.extend(right.spans);
    grid::fit(line, width)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::*;

    #[test]
    fn queue_rows_derive_labels_and_keep_their_deadline_without_room() {
        let mut state = state();
        state.apply(&frame(
            1,
            bingo_sdk::Event::QueueChanged {
                revision: 1,
                entries: (0..3)
                    .map(|n| bingo_sdk::QueueEntry {
                        intent: IntentId::from_raw(format!("q{n}")),
                        position: n + 1,
                        preview: format!("line {n}"),
                        steerable: n == 0,
                        origin: crate::origin(),
                    })
                    .collect(),
            },
        ));
        let now = scene().1;
        let mut arrivals = Arrivals::default();
        arrivals.observe(&[], &state.queue, now.instant);
        let shown = draw(&state, &arrivals, 80, now);
        assert!(shown.lines[0].to_string().contains("at the next step"));
        assert!(shown.lines[1].to_string().contains("queued 1"));
        assert!(shown.lines[2].to_string().contains("queued 2"));
        assert_eq!(draw(&state, &arrivals, 0, now).next, shown.next);
        assert!(shown.next.is_some());
        assert!(draw(&state, &arrivals, 80, later(now, 260)).next.is_none());
        state.apply(&frame(
            2,
            bingo_sdk::Event::QueueChanged {
                revision: 2,
                entries: vec![],
            },
        ));
        let empty = draw(&state, &arrivals, 80, now);
        assert!(empty.lines.is_empty() && empty.next.is_none());
    }
}
