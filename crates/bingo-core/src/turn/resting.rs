//! Where the context stands between rounds: the reading a session publishes
//! when it opens or changes model, on the ruler its next round measures with.

use bingo_sdk::{ContextUsage, SessionState};

use super::ModelChoice;
use super::ruler::Ruler;
use crate::context::{ContextView, estimate_tokens};

/// What the session holds now: the last measure, or the transcript's
/// estimate where it has grown past it since.
pub(crate) fn context_used(state: &SessionState) -> u64 {
    let estimate = estimate_tokens(&[], &ContextView::model_view(&state.transcript), &[]);
    // An unfinished turn can have grown since the last measured response.
    state
        .context
        .map_or(estimate, |context| context.used.max(estimate))
}

/// The reading for `model` before its next round, or `None` while its
/// window is only the fallback guess. A held context reads as the
/// endpoint's own last count (ADR-0055 §1).
pub(crate) fn reading(model: &ModelChoice, state: &SessionState) -> Option<ContextUsage> {
    if !model.window_known {
        return None;
    }
    let window = model.capabilities.context_window;
    let mut ruler = Ruler::new(window, model.max_tokens, model.endpoint.holds_context());
    Some(match state.context {
        Some(last) if ruler.holds() => ruler.reading(last.used, window),
        _ => ruler.measure(context_used(state)),
    })
}

/// Whether `reading` says something the clients do not already hold. A
/// window nobody knows clears the old one only when the model changed: the
/// last measure is otherwise still the best word on it.
pub(crate) fn restates(
    held: Option<ContextUsage>,
    reading: Option<ContextUsage>,
    switched: bool,
) -> bool {
    match (held, reading) {
        (None, None) => false,
        (Some(_), None) => switched,
        (None, Some(_)) => true,
        (Some(held), Some(reading)) => {
            (held.used, held.window, held.trigger)
                != (reading.used, reading.window, reading.trigger)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bingo_sdk::{ContextKind, ContextPart};

    fn usage(used: u64, window: u64) -> ContextUsage {
        ContextUsage {
            parts: None,
            used,
            window,
            trigger: window * 9 / 10,
        }
    }

    #[test]
    fn only_a_new_word_is_restated() {
        let measured = ContextUsage {
            parts: Some(
                [
                    ContextKind::System,
                    ContextKind::Tools,
                    ContextKind::Chat,
                    ContextKind::Pictures,
                ]
                .map(|name| ContextPart { name, tokens: 1 }),
            ),
            ..usage(14, 1_000)
        };
        assert!(restates(None, Some(usage(0, 1_000)), false));
        assert!(!restates(Some(measured), Some(usage(14, 1_000)), true));
        assert!(restates(Some(measured), Some(usage(14, 2_000)), false));
        assert!(!restates(None, None, true));
        assert!(!restates(Some(measured), None, false));
        assert!(restates(Some(measured), None, true));
    }
}
