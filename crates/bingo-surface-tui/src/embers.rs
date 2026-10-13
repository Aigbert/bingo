//! Token crossings leave brief light in the activity band's reserved air.
use std::time::{Duration, Instant};

pub const LIFE: Duration = Duration::from_millis(600);

pub fn crossings(before: u64, after: u64, now: Instant) -> Vec<Instant> {
    vec![now; (after / 64).saturating_sub(before / 64).min(4) as usize]
}

pub fn alive(births: &[Instant], now: Instant, motion: bool) -> Vec<Instant> {
    if !motion {
        return Vec::new();
    }
    births
        .iter()
        .copied()
        .filter(|born| now.saturating_duration_since(*born) < LIFE)
        .collect()
}

/// Streaming usage is not reported until the round ends. Four characters
/// per token is a motion estimate only; billed usage remains the fold's fact.
pub fn tokens(state: &bingo_sdk::SessionState) -> u64 {
    let Some(turn) = &state.turn else {
        return 0;
    };
    let characters: usize = state
        .transcript
        .items()
        .iter()
        .filter(|item| item.turn.as_ref() == Some(&turn.id))
        .map(|item| match &item.body {
            bingo_sdk::ItemBody::Assistant { text }
            | bingo_sdk::ItemBody::Reasoning { text, .. } => text.chars().count(),
            _ => 0,
        })
        .sum();
    (characters / 4) as u64
}

/// Only animation birth instants are retained; token counts come from the fold.
#[derive(Debug, Default)]
pub struct Embers(std::collections::BTreeMap<bingo_sdk::SessionId, Flight>);

#[derive(Debug)]
struct Flight {
    turn: bingo_sdk::TurnId,
    births: Vec<Instant>,
}

impl Embers {
    pub fn forget(&mut self, id: &bingo_sdk::SessionId) {
        self.0.remove(id);
    }
    pub fn observe(&mut self, before: u64, state: &bingo_sdk::SessionState, now: Instant) {
        let id = &state.summary.id;
        let Some(turn) = &state.turn else {
            self.0.remove(id);
            return;
        };
        let births = crossings(before, tokens(state), now);
        let flight = self.0.entry(id.clone()).or_insert_with(|| Flight {
            turn: turn.id.clone(),
            births: Vec::new(),
        });
        if flight.turn != turn.id {
            flight.turn = turn.id.clone();
            flight.births.clear();
        }
        flight.births = alive(&flight.births, now, true);
        flight.births.extend(births);
        let extra = flight.births.len().saturating_sub(4);
        flight.births.drain(..extra);
    }

    pub fn row(
        &self,
        state: &bingo_sdk::SessionState,
        now: crate::clock::Now,
        width: usize,
        foot: bool,
    ) -> ratatui::text::Line<'static> {
        let Some(flight) = self.0.get(&state.summary.id) else {
            return Default::default();
        };
        if foot
            || state
                .turn
                .as_ref()
                .is_none_or(|turn| turn.id != flight.turn)
        {
            return Default::default();
        }
        air(&flight.births, now.instant, now.motion, width)
    }
}

fn air(
    births: &[Instant],
    now: Instant,
    motion: bool,
    width: usize,
) -> ratatui::text::Line<'static> {
    use ratatui::text::{Line, Span};
    let mut spans = Vec::new();
    for (i, born) in alive(births, now, motion).into_iter().take(4).enumerate() {
        if i * 2 + 2 > width {
            break;
        }
        let age = now.saturating_duration_since(born).as_secs_f32() / LIFE.as_secs_f32();
        let glyph = if crate::theme::glyphs() == &crate::theme::ASCII {
            "."
        } else if age < 0.33 {
            "·"
        } else if age < 0.66 {
            "∙"
        } else {
            "˙"
        };
        spans.push(Span::raw(" "));
        spans.push(Span::styled(glyph, crate::theme::comet(age)));
    }
    Line::from(spans)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn tokens_cross_in_sixty_fours_and_leave_at_most_four_embers() {
        let now = Instant::now();
        assert!(crossings(0, 63, now).is_empty());
        assert_eq!(crossings(63, 64, now), vec![now]);
        assert_eq!(crossings(64, 1024, now), vec![now; 4]);
        assert!(crossings(128, 64, now).is_empty());
        assert!(alive(&[now], now + LIFE, true).is_empty());
        assert!(alive(&[now], now, false).is_empty());
    }
}
