use crate::{Transcript, TurnId};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// The most of a prompt's first line a turn carries; a row is narrower still.
pub const HEADLINE_CHARS: usize = 200;

/// One turn as a row: its headline only. The whole prompt stays in the
/// transcript, where `asked` reads it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct Turn {
    pub id: TurnId,
    pub headline: Option<String>,
    pub first: usize,
    pub last: usize,
}
impl Turn {
    pub fn label(&self) -> String {
        self.headline.clone().unwrap_or_else(|| self.id.to_string())
    }
    pub fn items(&self) -> usize {
        self.last - self.first + 1
    }
    /// The person's whole prompt for this turn, borrowed from the transcript.
    pub fn asked<'a>(&self, transcript: &'a Transcript) -> Option<&'a str> {
        let items = transcript.items().get(self.first..=self.last)?;
        items
            .iter()
            .filter(|item| item.turn.as_ref() == Some(&self.id))
            .find_map(|item| crate::asked(item))
    }
}

fn headline(asked: &str) -> String {
    crate::first_line(asked)
        .unwrap_or_default()
        .trim()
        .chars()
        .take(HEADLINE_CHARS)
        .collect()
}

/// Visible turns in transcript order; interstitial items belong to the preceding cut.
pub fn turns(transcript: &Transcript) -> Vec<Turn> {
    let mut out: Vec<Turn> = Vec::new();
    for (at, item) in transcript.items().iter().enumerate() {
        if let Some(id) = &item.turn {
            if out.last().is_none_or(|turn| &turn.id != id) {
                out.push(Turn {
                    id: id.clone(),
                    headline: None,
                    first: at,
                    last: at,
                });
            }
            if let (Some(turn), Some(asked)) = (out.last_mut(), crate::asked(item)) {
                turn.headline.get_or_insert_with(|| headline(asked));
            }
        }
        if let Some(turn) = out.last_mut() {
            turn.last = at;
        }
    }
    out
}
pub fn from(turns: &[Turn], wanted: &TurnId) -> Option<Vec<TurnId>> {
    let at = turns.iter().position(|turn| &turn.id == wanted)?;
    Some(turns[at..].iter().map(|turn| turn.id.clone()).collect())
}
