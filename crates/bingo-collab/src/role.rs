//! What a teammate is for, as the definition it was hired from describes it:
//! published once into the teammate's own journal when it starts, and read
//! back at the head of its stream. Nothing else keeps a copy of it.

use bingo_sdk::SessionState;
use serde_json::{Value, json};

/// The namespace of the plugin that hires teammates.
pub const PLUGIN: &str = "bingo.agents";

/// The kind the role is published under.
pub const KIND: &str = "role";

/// The one field of that payload; it is read by hand out of a persisted
/// journal, so it is written down once.
const TEXT: &str = "text";

pub fn payload(role: &str) -> Value {
    json!({ TEXT: role })
}

/// A teammate's role, or nothing for a session that was given none.
pub fn of_state(state: &SessionState) -> Option<String> {
    let role = state.extensions.get(PLUGIN)?.get(KIND)?[TEXT]
        .as_str()?
        .trim();
    (!role.is_empty()).then(|| role.to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The persisted shape: a journal written today is read by every later
    /// build.
    #[test]
    fn the_payload_is_one_text_field() {
        assert_eq!(payload("finds things"), json!({"text": "finds things"}));
    }

    #[test]
    fn a_published_role_reads_back_and_a_blank_one_is_none() {
        let event: bingo_sdk::Event = serde_json::from_str(include_str!(
            "../../bingo-sdk/fixtures/record/compat/summary-before-asking.json"
        ))
        .unwrap();
        let bingo_sdk::Event::SessionUpdated { summary } = event else {
            panic!("a summary fixture");
        };
        let mut state = SessionState::new(summary);
        assert_eq!(of_state(&state), None);
        state
            .extensions
            .entry(PLUGIN.into())
            .or_default()
            .insert(KIND.into(), payload("reads diffs and says what breaks"));
        assert_eq!(
            of_state(&state).as_deref(),
            Some("reads diffs and says what breaks")
        );
        state
            .extensions
            .entry(PLUGIN.into())
            .or_default()
            .insert(KIND.into(), payload("  "));
        assert_eq!(of_state(&state), None);
    }
}
