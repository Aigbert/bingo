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

#[test]
fn a_disabled_module_runtime_never_spawns_a_process() {
    use bingo_plugin_host::runtime::runtime_of;
    use bingo_sdk::PluginRuntime;
    let module: Manifest = serde_json::from_value(json!({"name":"fixture", "version":"1", "module":"main.ts"})).unwrap();
    assert_eq!(runtime_of(&module, true).unwrap(), PluginRuntime::Module);
    assert!(runtime_of(&module, false).unwrap_err().contains("fixture"));
    let process: Manifest = serde_json::from_value(json!({"name":"fixture", "version":"1", "entry":{"command":"node"}})).unwrap();
    for enabled in [true, false] { assert_eq!(runtime_of(&process, enabled).unwrap(), PluginRuntime::Process); }
}
