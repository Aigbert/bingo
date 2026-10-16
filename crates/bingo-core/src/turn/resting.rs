//! Where the context stands between rounds: the reading a session publishes
//! when it opens or changes model, on the ruler its next round measures with.

use bingo_sdk::{ContextUsage, SessionState};

use super::ruler::Ruler;
use super::{ModelChoice, TurnConfig};
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

/// Whether the model answering under `to` is another than under `from`.
pub(crate) fn switched(from: &TurnConfig, to: &TurnConfig) -> bool {
    model_key(from) != model_key(to)
}

fn model_key(config: &TurnConfig) -> Option<(&str, &str)> {
    config
        .model
        .as_ref()
        .map(|model| (model.endpoint.id(), model.id.as_str()))
}

/// What a session says about its context outside a round, if anything:
/// `Some(None)` clears a reading the clients hold, `None` says nothing.
pub(crate) fn restated(
    model: Option<&ModelChoice>,
    state: &SessionState,
    switched: bool,
) -> Option<Option<ContextUsage>> {
    let reading = model.and_then(|model| reading(model, state, switched));
    restates(state.context, reading, switched).then_some(reading)
}

/// The reading for `model` before its next round, or `None` while its
/// window is only the fallback guess. A held context reads as the
/// endpoint's own last count in the endpoint's own window, and as zero on a
/// model that has said nothing yet (ADR-0055 §1).
pub(super) fn reading(
    model: &ModelChoice,
    state: &SessionState,
    switched: bool,
) -> Option<ContextUsage> {
    if !model.window_known {
        return None;
    }
    let window = model.capabilities.context_window;
    let mut ruler = Ruler::new(window, model.max_tokens, model.endpoint.holds_context());
    let reading = match state.context.filter(|_| !switched) {
        Some(last) if ruler.holds() => ruler.reading(last.used, last.window),
        _ => ruler.measure(context_used(state)),
    };
    Some(with_parts(reading, state.context))
}

/// The last measure's breakdown still describes a request of the same size;
/// one of another size it would misreport, so that one has none.
fn with_parts(reading: ContextUsage, last: Option<ContextUsage>) -> ContextUsage {
    match last {
        Some(last) if last.used == reading.used => ContextUsage {
            parts: last.parts,
            ..reading
        },
        _ => reading,
    }
}

/// Whether `reading` says something the clients do not already hold. A
/// window nobody knows clears the old one only when the model changed: the
/// last measure is otherwise still the best word on it.
fn restates(held: Option<ContextUsage>, reading: Option<ContextUsage>, switched: bool) -> bool {
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

    #[test]
    fn a_breakdown_is_kept_only_for_a_request_of_its_size() {
        let parts = Some(
            [
                ContextKind::System,
                ContextKind::Tools,
                ContextKind::Chat,
                ContextKind::Pictures,
            ]
            .map(|name| ContextPart { name, tokens: 1 }),
        );
        let last = ContextUsage {
            parts,
            ..usage(14, 1_000)
        };
        assert_eq!(with_parts(usage(14, 2_000), Some(last)).parts, parts);
        assert_eq!(with_parts(usage(15, 1_000), Some(last)).parts, None);
        assert_eq!(with_parts(usage(14, 1_000), None).parts, None);
    }
}
