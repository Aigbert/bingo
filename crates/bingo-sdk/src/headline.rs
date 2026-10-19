//! A delivery's human-facing words, kept out of the model's content parts.
use crate::Tone;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct Headline {
    pub tone: Tone,
    /// Each part is one line of inline Markdown.
    pub parts: Vec<String>,
}
