//! What a command asks the surface to say: parts in their owner's inks, under one tone.
use crate::{Action, CommandOutcome, Tone, view::TextChild};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// The surface action a [`Say`] travels in.
pub const ACTION: &str = "Toast";

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct Say {
    /// Plain strings draw muted; a `Text` node keeps its own ink.
    pub parts: Vec<TextChild>,
    #[serde(default, skip_serializing_if = "Tone::is_neutral")]
    pub tone: Tone,
}

impl Say {
    pub fn new(parts: impl IntoIterator<Item = impl Into<TextChild>>) -> Self {
        Self {
            parts: parts.into_iter().map(Into::into).collect(),
            tone: Tone::Neutral,
        }
    }

    /// A finished thing: the good mark, then its words in one muted run.
    pub fn ok(text: impl Into<String>) -> Self {
        Self {
            tone: Tone::Good,
            ..Self::new([text.into()])
        }
    }

    pub fn action(&self) -> Action {
        Action {
            name: ACTION.into(),
            args: serde_json::to_value(self).unwrap_or_default(),
        }
    }

    /// A surface that draws toasts says it; any other client prints `fallback`.
    pub fn outcome(&self, fallback: impl Into<String>) -> CommandOutcome {
        CommandOutcome::Surface {
            action: self.action(),
            fallback: Some(fallback.into()),
        }
    }

    /// The words a client without inks reads.
    pub fn plain(&self) -> String {
        self.parts
            .iter()
            .map(TextChild::fold)
            .collect::<Vec<_>>()
            .join(" · ")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    #[test]
    fn a_said_toast_round_trips_its_parts_and_tone() {
        for wire in [
            json!({"parts":["rewound · 2 files restored"],"tone":"good"}),
            json!({"parts":[{"kind":"Text","children":["tool-fs on"],"color":"good","wrap":"truncate"},"from the next turn"]}),
        ] {
            let said: Say = serde_json::from_value(wire.clone()).unwrap();
            assert_eq!(serde_json::to_value(&said).unwrap(), wire);
            assert_eq!(said.action().args, wire);
        }
        assert_eq!(
            Say::ok("rewound").action().args,
            json!({"parts":["rewound"],"tone":"good"})
        );
        assert_eq!(
            Say::new(["paused", "check CI"]).plain(),
            "paused · check CI"
        );
    }
}
