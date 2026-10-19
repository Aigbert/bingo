use bingo_sdk::sheet::{Press, pressed};
use serde_json::{Value, json};

#[test]
fn sheet_presses_round_trip_without_interpreting_row_ids() {
    let cases: Vec<Value> =
        serde_json::from_str(include_str!("fixtures/sheet_press.json")).unwrap();
    let expected = [
        Press::Key("down".into()),
        Press::Pick("model:anthropic/opus".into()),
        Press::Pick("pick:row".into()),
        Press::Pick("enter".into()),
    ];
    for (value, expected) in cases.into_iter().zip(expected) {
        let press: Press = serde_json::from_value(value.clone()).unwrap();
        assert_eq!(press, expected);
        assert_eq!(serde_json::to_value(press).unwrap(), value);
        assert_eq!(pressed(&json!({"value":value})), Some(expected));
    }
}

#[test]
fn ambiguous_presses_are_rejected_and_spaces_are_keys() {
    assert!(pressed(&json!({"value":{"key":"up","pick":"row"}})).is_none());
    assert_eq!(
        pressed(&json!({"value":{"key":" "}})),
        Some(Press::Key("space".into()))
    );
    assert_eq!(
        pressed(&json!({"value":{"pick":" "}})),
        Some(Press::Pick(" ".into()))
    );
}
