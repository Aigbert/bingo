use bingo_sdk::catalogue;
use serde_json::json;

#[test]
fn registration_pins_identity_and_describes_the_grant() {
    let entry = catalogue::find("plugin.register").expect("plugin.register is a catalogue chain");
    let input = json!({"name":"fixture", "version":"1", "origin":"project",
        "runtime":"module", "tier":"tighten", "uses":["state"]});
    assert!((entry.valid_input)(&input));
    assert!((entry.valid_result)(&json!({"tier":"tighten", "uses":[]})));
    assert!(!(entry.valid_result)(&json!({"tier":"admin", "uses":[]})));
    assert_eq!(entry.pinned, ["/name", "/version", "/origin", "/runtime"]);
}
