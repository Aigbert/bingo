//! A plugin chooses a place; a surface chooses how it looks (ADR-0070).

use jiff::Timestamp;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::{ActionItem, ItemId, SessionState, Tone, View};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum Slot {
    Status {
        text: String,
        tone: Tone,
        #[schemars(with = "Option<String>")]
        until: Option<Timestamp>,
    },
    Activity {
        verb: Option<String>,
        view: View,
    },
    Rail {
        title: String,
        view: View,
    },
    Panel {
        title: String,
        view: View,
    },
    Mode {
        current: String,
        /// The first label is the resting mode, which a surface may leave unsaid.
        modes: Vec<ActionItem>,
    },
    Row {
        item: ItemId,
        mark: Option<char>,
        title: Option<String>,
        badge: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        caption: Option<String>,
        body: Option<View>,
        actions: Vec<ActionItem>,
    },
}

#[derive(Clone, Debug, PartialEq)]
pub struct Placed {
    pub plugin: String,
    /// Empty for a policy's configuration description.
    pub kind: String,
    pub slot: Slot,
}

/// Add placement to an object; a removal stays a removal.
pub fn with(mut payload: Value, slots: impl IntoIterator<Item = Slot>) -> Value {
    if let Some(object) = payload.as_object_mut() {
        let slots = slots
            .into_iter()
            .filter_map(|slot| serde_json::to_value(slot).ok())
            .collect();
        object.insert("slots".into(), Value::Array(slots));
    }
    payload
}

/// Read all three lanes in plugin and kind order, preserving each slot's order.
/// An unreadable slot is skipped without losing its readable neighbours.
pub fn placed(state: &SessionState) -> Vec<Placed> {
    let mut payloads: Vec<(&str, &str, &Value)> = [&state.signals, &state.extensions]
        .into_iter()
        .flat_map(|lane| {
            lane.iter().flat_map(|(plugin, kinds)| {
                kinds
                    .iter()
                    .map(move |(kind, payload)| (plugin.as_str(), kind.as_str(), payload))
            })
        })
        .chain(
            state
                .config
                .plugins
                .iter()
                .map(|(plugin, payload)| (plugin.as_str(), "", payload)),
        )
        .collect();
    payloads.sort_by(|a, b| (a.0, a.1).cmp(&(b.0, b.1)));
    payloads
        .into_iter()
        .flat_map(|(plugin, kind, payload)| {
            payload
                .get("slots")
                .and_then(Value::as_array)
                .into_iter()
                .flatten()
                .filter_map(move |value| {
                    serde_json::from_value(value.clone())
                        .ok()
                        .map(|slot| Placed {
                            plugin: plugin.into(),
                            kind: kind.into(),
                            slot,
                        })
                })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Event;
    use serde_json::json;

    fn slots() -> Vec<Slot> {
        serde_json::from_str(include_str!("../fixtures/slots/all.json")).unwrap()
    }

    fn state() -> SessionState {
        let Event::SessionUpdated { summary } =
            serde_json::from_str(include_str!("../fixtures/record/event/sessionUpdated.json"))
                .unwrap()
        else {
            panic!("session fixture")
        };
        SessionState::new(summary)
    }

    #[test]
    fn slot_variants_round_trip_the_golden_json() {
        let json: Value = serde_json::from_str(include_str!("../fixtures/slots/all.json")).unwrap();
        let generated = serde_json::to_value(slots()).unwrap();
        if std::env::var_os("BINGO_UPDATE_SCHEMA").is_some() {
            std::fs::write(
                concat!(env!("CARGO_MANIFEST_DIR"), "/fixtures/slots/all.json"),
                format!("{}\n", serde_json::to_string_pretty(&generated).unwrap()),
            )
            .unwrap();
        } else {
            assert_eq!(generated, json);
        }
        assert_eq!(slots().len(), 7);
    }

    #[test]
    fn row_caption_is_optional_in_the_schema_and_wire() {
        let schema = serde_json::to_value(schemars::schema_for!(Slot)).unwrap();
        let row = schema["oneOf"]
            .as_array()
            .unwrap()
            .iter()
            .find(|variant| variant["properties"]["kind"]["const"] == "row")
            .unwrap();
        assert!(row["properties"].get("caption").is_some());
        assert!(
            !row["required"]
                .as_array()
                .unwrap()
                .contains(&json!("caption"))
        );
        let rows = serde_json::to_value(slots()).unwrap();
        assert!(rows[5].get("caption").is_none());
        assert_eq!(rows[6]["caption"], "the wire format");
    }

    #[test]
    fn slot_with_keeps_the_record_and_leaves_null_alone() {
        assert_eq!(with(Value::Null, slots()), Value::Null);
        let payload = with(json!({"record": 42}), slots());
        assert_eq!(payload["record"], 42);
        assert_eq!(payload["slots"], serde_json::to_value(slots()).unwrap());
    }

    #[test]
    fn slot_placed_skips_only_the_unreadable_slot() {
        let mut state = state();
        let good = serde_json::to_value(&slots()[0]).unwrap();
        state.signals.entry("stranger".into()).or_default().insert(
            "live".into(),
            json!({"slots": [{"kind":"future"}, good, {"kind":"status"}]}),
        );
        assert_eq!(
            placed(&state),
            vec![Placed {
                plugin: "stranger".into(),
                kind: "live".into(),
                slot: slots().remove(0),
            }]
        );
    }

    #[test]
    fn slot_placed_orders_all_lanes_by_plugin_then_kind() {
        let mut state = state();
        let payload = with(json!({}), [slots().remove(0)]);
        state
            .signals
            .entry("z".into())
            .or_default()
            .insert("a".into(), payload.clone());
        state
            .signals
            .entry("a".into())
            .or_default()
            .insert("z".into(), payload.clone());
        state
            .extensions
            .entry("a".into())
            .or_default()
            .insert("b".into(), payload.clone());
        state.config.plugins.insert("a".into(), payload);
        let keys: Vec<_> = placed(&state)
            .into_iter()
            .map(|p| (p.plugin, p.kind))
            .collect();
        assert_eq!(
            keys,
            [("a", ""), ("a", "b"), ("a", "z"), ("z", "a")]
                .map(|(p, k)| (p.to_owned(), k.to_owned()))
        );
    }
}
