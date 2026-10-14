use super::*;
#[test]
fn fixtures_pin_each_rule_and_its_source_location() {
    let fixtures = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/validate");
    for rule in [
        "on-literal",
        "dollar-static",
        "state-key",
        "env-name",
        "uses",
        "state-contract",
        "prop-subset",
        "import",
    ] {
        let root = fixtures.join(rule);
        let manifest =
            serde_json::from_str(&std::fs::read_to_string(root.join("plugin.json")).unwrap())
                .unwrap();
        let actual = validate(&root, &manifest).unwrap();
        let expected: Vec<Finding> =
            serde_json::from_str(&std::fs::read_to_string(root.join("expected.json")).unwrap())
                .unwrap();
        assert_eq!(actual.findings, expected, "{rule}");
    }
}
