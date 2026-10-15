use crate::{Transcript, TurnId};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct Turn {
    pub id: TurnId,
    pub asked: Option<String>,
    pub first: usize,
    pub last: usize,
}
impl Turn {
    pub fn label(&self) -> String {
        self.asked
            .as_deref()
            .and_then(|s| s.lines().next())
            .map(str::trim)
            .map(str::to_owned)
            .unwrap_or_else(|| self.id.to_string())
    }
    pub fn items(&self) -> usize {
        self.last - self.first + 1
    }
}

/// Visible turns in transcript order; interstitial items belong to the preceding cut.
pub fn turns(transcript: &Transcript) -> Vec<Turn> {
    let mut out: Vec<Turn> = Vec::new();
    for (at, item) in transcript.items().iter().enumerate() {
        if let Some(id) = &item.turn {
            if out.last().is_none_or(|turn| &turn.id != id) {
                out.push(Turn {
                    id: id.clone(),
                    asked: None,
                    first: at,
                    last: at,
                });
            }
            if let (Some(turn), Some(asked)) = (out.last_mut(), crate::asked(item)) {
                turn.asked.get_or_insert_with(|| asked.to_owned());
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
