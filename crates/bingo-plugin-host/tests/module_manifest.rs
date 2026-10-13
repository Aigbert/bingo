//! ADR-0078: a module stands where a process manifest names its entry.
#![allow(clippy::expect_used)]

use bingo_plugin_host::manifest::Manifest;

#[test]
fn a_module_manifest_needs_no_process_entry() {
    let manifest: Manifest = serde_json::from_str(include_str!("fixtures/module.json"))
        .expect("ADR-0078 permits a module without a process entry");
    assert_eq!(manifest.name, "hello");
}
