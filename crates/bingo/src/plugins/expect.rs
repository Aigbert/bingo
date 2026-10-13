//! Expectations read the same folded transcript and placements as a surface.
use super::schema::Expectation;
use bingo_sdk::{ContentPart, ItemBody, SessionState, slot};

pub fn satisfied(expected: &Expectation, state: &SessionState, notices: &[String]) -> bool {
    match expected {
        Expectation::Slot { slot: key, contains } => slot::placed(state).iter().any(|placed| {
            &placed.kind == key && serde_json::to_value(&placed.slot).is_ok_and(|value| value.to_string().contains(contains))
        }),
        Expectation::Tool { name, contains } => calls(state, name).any(|(_, text)| text.contains(contains)),
        Expectation::Denied { name } => calls(state, name).any(|(error, text)| error && text.to_lowercase().contains("denied")),
        Expectation::Notice { contains } => notices.iter().any(|text| text.contains(contains))
            || state.transcript.items().iter().any(|item| matches!(&item.body, ItemBody::Notice { text, .. } if text.contains(contains))),
    }
}

fn calls<'a>(state: &'a SessionState, name: &'a str) -> impl Iterator<Item = (bool, String)> + 'a {
    state
        .transcript
        .items()
        .iter()
        .filter_map(move |item| match &item.body {
            ItemBody::ToolCall {
                name: called,
                output: Some(output),
                ..
            } if called == name => Some((
                output.is_error,
                output
                    .parts
                    .iter()
                    .filter_map(ContentPart::as_text)
                    .collect(),
            )),
            _ => None,
        })
}

pub fn shown(state: &SessionState, notices: &[String]) -> String {
    format!(
        "slots: {:?}; items: {:?}; notices: {notices:?}",
        slot::placed(state),
        state.transcript.items()
    )
}
