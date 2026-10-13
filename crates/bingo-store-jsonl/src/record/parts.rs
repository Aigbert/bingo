//! The known content-part paths; plugin data and unknown bodies are not visited.

use super::raw::{self, Changed, Rewrite};
use bingo_sdk::{Event, ItemBody};
use serde_json::value::RawValue;

pub(super) fn frame(raw: &RawValue, writing: bool, picture: &mut Rewrite<'_>) -> Changed {
    raw::field(raw, "event", &mut |raw| event(raw, writing, picture))
}

fn event(raw: &RawValue, writing: bool, picture: &mut Rewrite<'_>) -> Changed {
    if !matches!(
        raw::tag(raw, "type").as_deref(),
        Some("itemStarted" | "itemUpdated" | "itemCompleted")
    ) {
        return Ok(None);
    }
    if writing && !known_event(raw) {
        return Ok(None);
    }
    let changed = raw::field(raw, "item", &mut |raw| item(raw, writing, picture))?;
    Ok(changed.filter(|value| writing || known_event(value)))
}

fn known_event(raw: &RawValue) -> bool {
    matches!(serde_json::from_str::<Event>(raw.get()), Ok(event) if !matches!(event, Event::Unknown { .. }))
}

fn item(raw: &RawValue, writing: bool, picture: &mut Rewrite<'_>) -> Changed {
    raw::field(raw, "body", &mut |raw| body(raw, writing, picture))
}

fn body(raw: &RawValue, writing: bool, picture: &mut Rewrite<'_>) -> Changed {
    if writing && !known_body(raw) {
        return Ok(None);
    }
    let changed = match raw::tag(raw, "kind").as_deref() {
        Some("user") => content(raw, picture)?,
        Some("toolCall") => raw::field(raw, "output", &mut |raw| content(raw, picture))?,
        _ => None,
    };
    Ok(changed.filter(|value| writing || known_body(value)))
}

fn known_body(raw: &RawValue) -> bool {
    matches!(serde_json::from_str::<ItemBody>(raw.get()), Ok(body) if !matches!(body, ItemBody::Unknown { .. }))
}

fn content(raw: &RawValue, picture: &mut Rewrite<'_>) -> Changed {
    raw::field(raw, "parts", &mut |raw| {
        raw::array(raw, &mut |raw| part(raw, picture))
    })
}

fn part(raw: &RawValue, picture: &mut Rewrite<'_>) -> Changed {
    match raw::tag(raw, "type").as_deref() {
        Some("image") => picture(raw),
        Some("toolResult") => content(raw, picture),
        _ => Ok(None),
    }
}

pub(super) fn checkpoint(raw: &RawValue, writing: bool, picture: &mut Rewrite<'_>) -> Changed {
    raw::field(raw, "state", &mut |raw| {
        raw::field(raw, "transcript", &mut |raw| {
            raw::field(raw, "items", &mut |raw| {
                raw::array(raw, &mut |raw| item(raw, writing, picture))
            })
        })
    })
}
