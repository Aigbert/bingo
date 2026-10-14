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

#[test]
fn existing_examples_and_registered_callback_parameters_validate_cleanly() {
    let workspace = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    for relative in [
        "examples/plugins/ts-status",
        "examples/plugins/tool-call",
        "examples/plugins/band",
        "examples/plugins/pane",
        "crates/bingo/tests/fixtures/mods/counter",
    ] {
        let root = workspace.join(relative);
        let manifest =
            serde_json::from_str(&std::fs::read_to_string(root.join("plugin.json")).unwrap())
                .unwrap();
        assert!(
            validate(&root, &manifest).unwrap().findings.is_empty(),
            "{relative}"
        );
    }
}

#[test]
fn passing_a_noun_or_method_cannot_hide_an_alias() {
    let result = checked(
        "export const register = on => on('*', $ => { helper($); helper($.state); helper($.state.get); });",
        "",
    );
    assert_eq!(
        result
            .findings
            .iter()
            .filter(|f| f.rule == Rule::DollarStatic)
            .count(),
        2
    );
}

#[test]
fn noun_data_reads_are_values_but_noun_and_method_aliases_are_not() {
    let values = checked(
        "export const register = on => on('session.start', $ => { const name = $.plugin.name; const version = $.plugin.version; const root = $.plugin.root; helper($.plugin.name); $.plugin.name.toUpperCase(); const first = $.plugin.name[0]; });",
        "",
    );
    assert!(values.valid(), "{:?}", values.findings);
    assert!(values.calls.is_empty(), "{:?}", values.calls);
    let aliases = checked(
        "export const register = on => on('session.start', $ => { const api = $; const noun = $.plugin; const method = $.state.get; const unknown = $.plugin.unknown; });",
        "",
    );
    assert_eq!(
        aliases
            .findings
            .iter()
            .filter(|f| f.rule == Rule::DollarStatic)
            .count(),
        4
    );
}

#[tokio::test]
async fn erased_type_imports_validate_and_load_with_the_same_boundary() {
    let root = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(root.path().join("hooks")).unwrap();
    std::fs::create_dir(root.path().join("types")).unwrap();
    std::fs::write(
        root.path().join("types/index.d.ts"),
        "export interface Shape { label: string }\nexport interface Other {}\n",
    )
    .unwrap();
    for import in [
        "import type { Shape } from '../types';",
        "import { type Shape, type Other } from '../types';",
        "import type { Shape } from '../types/index.d.ts';",
        "import type { Shape } from 'unavailable-types';",
    ] {
        let source =
            format!("{import}\nexport const register = on => on('session.start', () => 'loaded');");
        std::fs::write(root.path().join("hooks/main.ts"), &source).unwrap();
        let manifest = serde_json::from_value(
            serde_json::json!({"name":"fixture","version":"1","module":"hooks/main.ts"}),
        )
        .unwrap();
        assert!(
            validate(root.path(), &manifest).unwrap().valid(),
            "{import}"
        );
        let stripped = crate::strip::strip(Path::new("hooks/main.ts"), &source).unwrap();
        assert!(!stripped.contains("import"), "{stripped}");
        let engine = crate::engine::Engine::start(
            root.path(),
            "hooks/main.ts",
            serde_json::json!({}),
            Default::default(),
        )
        .await
        .unwrap();
        let mut call = engine
            .call(
                "session.start",
                serde_json::json!({}),
                serde_json::json!({"session":"s"}),
            )
            .unwrap();
        assert_eq!((&mut call.result).await.unwrap().unwrap(), "loaded");
        engine.close().await;
    }
}

#[tokio::test]
async fn directory_value_imports_resolve_but_declarations_never_execute() {
    let root = tempfile::tempdir().unwrap();
    std::fs::create_dir(root.path().join("helper")).unwrap();
    std::fs::write(
        root.path().join("helper/index.ts"),
        "export const label = 'directory';",
    )
    .unwrap();
    std::fs::write(root.path().join("main.ts"), "import { label } from './helper'; export const register = on => on('session.start', () => label);").unwrap();
    let manifest = serde_json::from_value(
        serde_json::json!({"name":"fixture","version":"1","module":"main.ts"}),
    )
    .unwrap();
    assert!(validate(root.path(), &manifest).unwrap().valid());
    let engine = crate::engine::Engine::start(
        root.path(),
        "main.ts",
        serde_json::json!({}),
        Default::default(),
    )
    .await
    .unwrap();
    let mut call = engine
        .call(
            "session.start",
            serde_json::json!({}),
            serde_json::json!({"session":"s"}),
        )
        .unwrap();
    assert_eq!((&mut call.result).await.unwrap().unwrap(), "directory");
    engine.close().await;
    std::fs::rename(
        root.path().join("helper/index.ts"),
        root.path().join("helper/index.d.ts"),
    )
    .unwrap();
    for specifier in ["./helper", "./helper/index.d.ts"] {
        std::fs::write(root.path().join("main.ts"), format!("import {{ label }} from '{specifier}'; export const register = on => on('session.start', () => label);")).unwrap();
        assert!(
            validate(root.path(), &manifest)
                .unwrap_err()
                .contains("declaration")
        );
        assert!(
            crate::load::Root::new(root.path(), 4096)
                .unwrap()
                .source("helper/index.d.ts")
                .unwrap_err()
                .contains("declaration")
        );
        assert!(
            crate::engine::Engine::start(
                root.path(),
                "main.ts",
                serde_json::json!({}),
                Default::default()
            )
            .await
            .is_err()
        );
    }
}

#[test]
fn module_constant_matchers_are_static_but_computed_matchers_are_not() {
    for source in [
        "const pane = 'activity'; export const register = on => on('ui.render', {component:'Pane',requestId:pane}, () => {});",
        "const match = {component:'Pane',requestId:'activity'}; export const register = on => on('ui.render', match, () => {});",
    ] {
        let result = checked(source, "");
        assert!(
            !result.findings.iter().any(|f| f.rule == Rule::OnLiteral),
            "{:?}",
            result.findings
        );
    }
    for source in [
        "const match = {component:'Pane'}; match.component = compute(); export const register = on => on('ui.render', match, () => {});",
        "let match = {component:'Pane'}; export const register = on => on('ui.render', match, () => {});",
        "const match = compute(); export const register = on => on('ui.render', match, () => {});",
        "const match = {requestId: compute()}; export const register = on => on('ui.render', match, () => {});",
        "const key = 'component'; const match = {[key]:'Pane'}; export const register = on => on('ui.render', match, () => {});",
        "const base = {component:'Pane'}; const match = {...base}; export const register = on => on('ui.render', match, () => {});",
        "const match = {component:'Pane'}; export const register = on => { const match = compute(); on('ui.render', match, () => {}); };",
    ] {
        assert!(
            checked(source, "")
                .findings
                .iter()
                .any(|f| f.rule == Rule::OnLiteral),
            "{source}"
        );
    }
}

#[test]
fn object_atom_keys_use_the_same_declared_key_and_owner() {
    let result = checked(
        "import {atom} from 'bingo'; const value = atom({plugin:'fixture',key:'count'} as const, 0);",
        "",
    );
    assert!(result.keys.contains("state:count"), "{:?}", result.findings);
    assert!(!result.findings.iter().any(|f| f.rule == Rule::StateKey));
    for key in [
        "{plugin:'other',key:'count'}",
        "{plugin:owner,key:'count'}",
        "{plugin:'fixture',key:name}",
        "{plugin:'fixture',key:'count',extra:true}",
        "{...key}",
    ] {
        let result = checked(
            &format!("import {{atom}} from 'bingo'; const value = atom({key}, 0);"),
            "",
        );
        assert!(
            result.findings.iter().any(|f| f.rule == Rule::StateKey),
            "{key}"
        );
    }
}
