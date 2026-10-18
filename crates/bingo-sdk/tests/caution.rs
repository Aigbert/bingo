use bingo_sdk::{Caution, InteractionKind, Preview, Stakes};

#[test]
fn string_and_object_cautions_read_from_fixtures() {
    for (wire, act) in [
        (
            include_str!("../fixtures/record/compat/permission-string-caution.json"),
            None,
        ),
        (
            include_str!("../fixtures/record/compat/permission-caution.json"),
            Some("force-push"),
        ),
    ] {
        let kind: InteractionKind = serde_json::from_str(wire).unwrap();
        let InteractionKind::Permission { caution, .. } = &kind else {
            panic!("permission");
        };
        assert_eq!(
            caution,
            &Some(Caution {
                reason: "rewrites remote history".into(),
                act: act.map(str::to_owned),
            })
        );
        let written = serde_json::to_value(kind).unwrap();
        assert_eq!(written["caution"]["reason"], "rewrites remote history");
        assert_eq!(written["caution"]["act"].as_str(), act);
    }
}

#[test]
fn command_stakes_round_trip_and_old_commands_omit_them() {
    let wire = include_str!("../fixtures/record/compat/command-stakes.json");
    let preview: Preview = serde_json::from_str(wire).unwrap();
    assert!(
        matches!(&preview, Preview::Command { stakes: Some(Stakes { line, items }), .. }
        if line == "drops 1 commit that only origin/dev has"
        && items == &["4e1a9c2 wip: queue experiments"])
    );
    assert_eq!(serde_json::to_string(&preview).unwrap(), wire.trim_end());
    let old = r#"{"kind":"command","command":"cargo test","cwd":"/work"}"#;
    let preview: Preview = serde_json::from_str(old).unwrap();
    assert!(matches!(preview, Preview::Command { stakes: None, .. }));
    assert_eq!(serde_json::to_string(&preview).unwrap(), old);
}

#[test]
fn caution_and_stakes_schema_fields_are_optional() {
    let schema = serde_json::to_value(schemars::schema_for!(InteractionKind)).unwrap();
    let permission = schema["oneOf"]
        .as_array()
        .unwrap()
        .iter()
        .find(|variant| variant["properties"]["kind"]["const"] == "permission")
        .unwrap();
    assert!(
        !permission["required"]
            .as_array()
            .unwrap()
            .contains(&serde_json::json!("caution"))
    );
    assert_eq!(
        permission["properties"]["caution"]["anyOf"][0]["$ref"],
        "#/$defs/Caution"
    );
    assert_eq!(
        schema["$defs"]["Caution"]["required"],
        serde_json::json!(["reason"])
    );
    let command = schema["$defs"]["Preview"]["oneOf"]
        .as_array()
        .unwrap()
        .iter()
        .find(|variant| variant["properties"]["kind"]["const"] == "command")
        .unwrap();
    assert!(
        !command["required"]
            .as_array()
            .unwrap()
            .contains(&serde_json::json!("stakes"))
    );
    assert_eq!(
        command["properties"]["stakes"]["anyOf"][0]["$ref"],
        "#/$defs/Stakes"
    );
}
