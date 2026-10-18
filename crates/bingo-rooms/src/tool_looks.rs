use super::*;
use bingo_sdk::{Contribution, Env, Plugin, Registrar};
use serde_json::{Value, json};

#[test]
fn every_tool_declares_its_look() {
    let mut registrar = Registrar::new(
        "look-test",
        json!({"experience":{"enabled":true},"schedule":{"wakes":true}}),
        Env::rooted("."),
        Default::default(),
    );
    RoomsPlugin::default().register(&mut registrar).unwrap();
    let actual: serde_json::Map<String, Value> = registrar
        .into_contributions()
        .iter()
        .filter_map(|entry| match entry {
            Contribution::Tool(tool) => {
                let spec = tool.spec();
                let look = spec.look.as_ref().unwrap();
                let normal = bingo_sdk::look::row(&spec.name, &Value::Null, Some(look), false);
                let ascii = bingo_sdk::look::row(&spec.name, &Value::Null, Some(look), true);
                assert_eq!(normal.mark, look.mark);
                assert_eq!(ascii.mark, look.ascii.unwrap_or(look.mark));
                assert!(ascii.mark.is_ascii());
                Some((spec.name, serde_json::to_value(spec.look).unwrap()))
            }
            _ => None,
        })
        .collect();
    let expected: Value = serde_json::from_str(include_str!("tool_looks.json")).unwrap();
    assert_eq!(Value::Object(actual), expected);
}
