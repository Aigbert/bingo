//! Client facts for reading a transcript by turns.
use crate::{
    Action, SessionId, TurnId,
    transcript::{Match, Turn},
};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct BrowseProps {
    pub session: SessionId,
    pub selected: Option<TurnId>,
    pub query: String,
    pub matches: Vec<Match>,
    pub turns: Vec<Turn>,
    pub current: usize,
    pub action: Action,
    pub confirm: bool,
}
