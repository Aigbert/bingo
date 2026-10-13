//! ADR-0078: exactly one launch form, in one manifest contract.
#![allow(clippy::expect_used, clippy::unwrap_used)]
use bingo_plugin_host::manifest::Manifest;

#[test]
fn entry_only_and_module_only_round_trip() {
    for source in [
        include_str!("fixtures/entry.json"),
        include_str!("fixtures/module.json"),
    ] {
        let manifest: Manifest = serde_json::from_str(source).expect("one launch form");
        let value = serde_json::to_value(manifest).unwrap();
        let original: serde_json::Value = serde_json::from_str(source).unwrap();
        assert_eq!(value.get("module"), original.get("module"));
        assert_eq!(
            value.get("entry").is_some(),
            original.get("entry").is_some()
        );
    }
}

#[test]
fn both_and_neither_name_the_plugin_and_launch_forms() {
    for source in [
        include_str!("fixtures/both.json"),
        include_str!("fixtures/neither.json"),
    ] {
        let error = serde_json::from_str::<Manifest>(source)
            .expect_err("exactly one launch form")
            .to_string();
        for word in ["hello", "entry", "module", "exactly one"] {
            assert!(error.contains(word), "{error}");
        }
    }
}
