use bingo_sdk::Preview;
use serde_json::json;

#[test]
fn rough_edges_a_command_preview_without_description_keeps_its_wire_shape() {
    let value = json!({"kind":"command","command":"echo hi","cwd":"/work"});
    let preview: Preview = serde_json::from_value(value.clone()).unwrap();
    assert_eq!(serde_json::to_value(preview).unwrap(), value);
}

#[test]
fn rough_edges_a_command_preview_round_trips_its_description() {
    let value = json!({"kind":"command","command":"echo hi","cwd":"/work","description":"Print a greeting"});
    let preview: Preview = serde_json::from_value(value.clone()).unwrap();
    assert_eq!(serde_json::to_value(preview).unwrap(), value);
}
