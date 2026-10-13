use bingo_plugin_host::manifest::Manifest;
use serde_json::json;
#[test]
fn manifest_keeps_declared_nouns_in_both_launch_forms() {
    for launch in [json!({"module":"main.ts"}), json!({"entry":{"command":"node"}})] {
        let mut value = json!({"name":"fixture", "version":"1", "uses":["state", "model"]});
        value.as_object_mut().unwrap().extend(launch.as_object().unwrap().clone());
        let manifest: Manifest = serde_json::from_value(value.clone()).unwrap();
        assert_eq!(serde_json::to_value(manifest).unwrap(), value);
    }
}
