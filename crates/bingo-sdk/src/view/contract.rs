use super::*;
use serde_json::json;

#[test]
fn elements_round_trip_and_fold_the_golden_nodes() {
    let cases: Vec<(Value, String)> =
        serde_json::from_str(include_str!("../../fixtures/views/elements.json")).unwrap();
    for (wire, fold) in cases {
        let view: View = serde_json::from_value(wire.clone()).unwrap();
        assert!(!matches!(view, View::Custom { .. }), "unlearned: {wire}");
        assert_eq!(view.fold(), fold, "{wire}");
        assert_eq!(serde_json::to_value(&view).unwrap(), wire);
    }
}

#[test]
fn legacy_nodes_read_into_the_canonical_vocabulary() {
    for (wire, kind, fold) in [
        (json!({"kind":"text","text":"old"}), "Text", "old"),
        (json!({"kind":"code","text":"x","lang":"rust"}), "Code", "x"),
        (
            json!({"kind":"stack","children":[{"kind":"text","text":"a"}]}),
            "Box",
            "a",
        ),
        (
            json!({"kind":"columns","children":[{"kind":"text","text":"a"}]}),
            "Box",
            "a",
        ),
    ] {
        let view: View = serde_json::from_value(wire).unwrap();
        assert_eq!(view.fold(), fold);
        assert_eq!(serde_json::to_value(view).unwrap()["kind"], kind);
    }
}
