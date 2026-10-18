use super::*;
use serde_json::{Value, json};

#[test]
fn every_tool_declares_its_look() {
    let looks = bingo_sdk::testing::tool_looks(&BashPlugin::default(), json!({})).unwrap();
    let expected: Value = serde_json::from_str(include_str!("tool_looks.json")).unwrap();
    assert_eq!(serde_json::to_value(looks).unwrap(), expected);
}
