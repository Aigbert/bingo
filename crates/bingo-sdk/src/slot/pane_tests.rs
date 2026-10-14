use super::*;
use serde_json::{Value, json};

fn pane() -> Value {
    json!({"kind":"pane","id":"counter","title":"Counter","focus":true,
        "closeOnEscape":true,"holdToasts":false,"asked":true,"rows":8,
        "view":{"kind":"Text","children":["counter"]}})
}

#[test]
fn pane_places_and_sizes_survive_the_wire() {
    for place in [
        json!(["side", "above"]),
        json!([{"rect":{
        "anchor":"bottomRight","width":"40%","height":"30%"}}]),
    ] {
        let mut wire = pane();
        wire["place"] = place.clone();
        wire["size"] = json!({"rows":{"min":4,"ideal":8,"max":12},
            "columns":{"min":48,"ideal":64,"max":"50%"}});
        let slot: Slot = serde_json::from_value(wire.clone()).unwrap();
        assert!(slot.published_ok());
        let written = serde_json::to_value(slot).unwrap();
        assert_eq!(written["place"], place);
        assert_eq!(written["size"], wire["size"]);
        assert!(written.get("rows").is_none());
    }
}

#[test]
fn legacy_pane_rows_become_one_size_and_default_above() {
    let slot: Slot = serde_json::from_value(pane()).unwrap();
    let wire = serde_json::to_value(slot).unwrap();
    assert_eq!(wire["place"], json!(["above"]));
    assert_eq!(wire["size"]["rows"]["ideal"], 8);
    assert!(wire.get("rows").is_none());
}

#[test]
fn published_panes_refuse_invalid_lengths_and_reversed_bounds() {
    for invalid in [
        json!({"place":[{"rect":{"width":"101%","height":8}}]}),
        json!({"size":{"rows":{"min":10,"ideal":8,"max":4}}}),
        json!({"size":{"columns":{"min":"80%","ideal":"90%","max":"50%"}}}),
    ] {
        let mut wire = pane();
        wire.as_object_mut()
            .unwrap()
            .extend(invalid.as_object().unwrap().clone());
        let slot = serde_json::from_value::<Slot>(wire);
        assert!(slot.is_err() || !slot.unwrap().published_ok());
    }
}
