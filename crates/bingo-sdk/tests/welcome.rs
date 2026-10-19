#![allow(clippy::unwrap_used, clippy::expect_used)]
use bingo_sdk::component::Props;
use serde_json::json;
#[test]
fn welcome_exposes_its_clock_and_recent_selection() {
    let all: Vec<serde_json::Value> =
        serde_json::from_str(include_str!("../src/component/props.json")).unwrap();
    let mut wire = all
        .into_iter()
        .find(|v| v["component"] == "Welcome")
        .unwrap();
    wire["props"]["startedAtMs"] = json!(1234);
    wire["props"]["recent"] = json!([]);
    wire["props"]["selected"] = json!(2);
    let props: Props = serde_json::from_value(wire.clone()).unwrap();
    assert_eq!(serde_json::to_value(props).unwrap(), wire);
}
