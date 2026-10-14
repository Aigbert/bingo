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

fn checked(source: &str, helper: &str) -> Validation {
    let root = tempfile::tempdir().unwrap();
    std::fs::write(root.path().join("main.ts"), source).unwrap();
    std::fs::write(root.path().join("helper.ts"), helper).unwrap();
    let manifest = serde_json::from_value(
        serde_json::json!({"name":"fixture","version":"1","module":"main.ts","uses":["state"]}),
    )
    .unwrap();
    validate(root.path(), &manifest).unwrap()
}
#[test]
fn reads_imported_helpers_and_resolves_const_patterns_by_binding() {
    let result = checked(
        "import { helper } from './helper.ts'; const event = 'session.start'; export const register = on => on(event, $ => helper($));",
        "export const helper = $ => $.state.get(variable);",
    );
    assert!(result.events.contains("session.start"));
    assert!(result.calls.contains("state.get"));
    assert!(
        result
            .findings
            .iter()
            .any(|f| f.file == "helper.ts" && f.rule == Rule::StateKey)
    );
    assert!(!result.findings.iter().any(|f| f.rule == Rule::DollarStatic));
    let shadowed = checked(
        "const event = 'session.start'; export const register = on => { let event = compute(); on(event, () => {}); };",
        "",
    );
    assert!(shadowed.findings.iter().any(|f| f.rule == Rule::OnLiteral));
}
#[test]
fn aliases_and_dynamic_matchers_are_errors_but_missing_contract_is_warning() {
    let result = checked(
        "export const register = on => on('*', matcher, $ => { const alias = $; const bag = {$}; const read = $.state.get; });",
        "",
    );
    assert_eq!(
        result
            .findings
            .iter()
            .filter(|f| f.rule == Rule::DollarStatic)
            .count(),
        3
    );
    assert!(result.findings.iter().any(|f| f.rule == Rule::OnLiteral));
    assert!(
        result
            .findings
            .iter()
            .any(|f| f.rule == Rule::StateContract && f.severity == Severity::Warning)
    );
}
