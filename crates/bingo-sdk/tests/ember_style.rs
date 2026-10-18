use bingo_sdk::View;
use serde_json::json;

#[test]
fn ember_styles_round_trip_without_losing_the_motion_contract() {
    for motion in [
        json!({"kind":"breathe","with":"coal","periodMs":1800,"depth":0.8}),
        json!({"kind":"spin"}),
        json!({"kind":"fade","from":"glow","atMs":1234,"ms":420}),
    ] {
        let value = json!({
            "kind":"Box", "ground":"well",
            "children":[{"kind":"Text", "color":"ember", "ground":"you",
                         "motion":motion, "children":["hello"]}]
        });
        let view: View = serde_json::from_value(value.clone()).unwrap();
        assert_eq!(serde_json::to_value(view).unwrap(), value);
    }
}

#[test]
fn unknown_motion_keeps_the_view_readable() {
    let view: View = serde_json::from_value(json!({
        "kind":"Text", "motion":{"kind":"shimmer"}, "children":["hello"]
    }))
    .unwrap();
    assert_eq!(view.fold(), "hello");
}

#[test]
fn unknown_motion_is_not_an_authoring_variant() {
    let schema = serde_json::to_value(schemars::schema_for!(bingo_sdk::view::TextMotion)).unwrap();
    let kinds: Vec<_> = schema["oneOf"]
        .as_array()
        .unwrap()
        .iter()
        .map(|variant| variant["properties"]["kind"]["const"].as_str().unwrap())
        .collect();
    assert_eq!(kinds, ["breathe", "spin", "fade"]);
}
