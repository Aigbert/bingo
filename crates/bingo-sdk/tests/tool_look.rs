use bingo_sdk::{Reads, ToolLook, ToolSpec};
use serde_json::{Value, json};

#[test]
fn a_tool_look_round_trips_its_fixture() {
    let fixture: Value = serde_json::from_str(include_str!("../fixtures/tool-look.json")).unwrap();
    let spec: ToolSpec = serde_json::from_value(fixture.clone()).unwrap();
    let look = spec.look.as_ref().unwrap();
    assert_eq!(look.mark, '¶');
    assert_eq!(look.ascii, Some('<'));
    assert_eq!(look.target.as_ref().unwrap().reads, Reads::Path);
    assert_eq!(serde_json::to_value(spec).unwrap(), fixture);
}

/// A look may name the input field that holds a principal the call
/// addresses; a surface draws the row in that principal's colour.
#[test]
fn a_look_naming_a_principal_round_trips_its_fixture() {
    let fixture: Value =
        serde_json::from_str(include_str!("../fixtures/tool-look-who.json")).unwrap();
    let spec: ToolSpec = serde_json::from_value(fixture.clone()).unwrap();
    let look = spec.look.as_ref().unwrap();
    assert_eq!(look.who.as_deref(), Some("to"));
    assert_eq!(serde_json::to_value(&spec).unwrap(), fixture);
    let built = ToolLook::new('@', "send")
        .target("message", Reads::Text)
        .who("to");
    assert_eq!(spec.look, Some(built));
}

#[test]
fn a_look_without_a_principal_says_nothing_of_one() {
    let look: ToolLook = serde_json::from_value(json!({"mark":"¶", "verb":"read"})).unwrap();
    assert!(look.who.is_none());
    assert_eq!(
        serde_json::to_value(look).unwrap(),
        json!({"mark":"¶", "verb":"read"})
    );
}

#[test]
fn an_old_spec_has_no_look_on_the_wire() {
    let fixture = json!({"name":"Old", "description":"Old tool", "inputSchema":{}});
    let spec: ToolSpec = serde_json::from_value(fixture.clone()).unwrap();
    assert!(spec.look.is_none());
    assert_eq!(serde_json::to_value(spec).unwrap(), fixture);
}

#[test]
fn the_look_schema_requires_one_character_and_a_verb() {
    let schema = serde_json::to_value(schemars::schema_for!(ToolLook)).unwrap();
    assert_eq!(schema["required"], json!(["mark", "verb"]));
    assert_eq!(schema["properties"]["mark"]["minLength"], 1);
    assert_eq!(schema["properties"]["mark"]["maxLength"], 1);
    assert_eq!(schema["properties"]["verb"]["minLength"], 1);
}

#[test]
fn malformed_looks_are_refused_with_a_message() {
    for (look, message) in [
        (json!({"mark":"xx", "verb":"read"}), "character"),
        (json!({"mark":"", "verb":"read"}), "character"),
        (json!({"mark":"¶", "verb":""}), "verb"),
        (json!({"mark":"¶", "verb":"   "}), "verb"),
    ] {
        let error = serde_json::from_value::<ToolLook>(look).unwrap_err();
        assert!(error.to_string().contains(message), "{error}");
    }
}

#[test]
fn catalogue_accessors_read_typed_facts_and_fail_closed() {
    use bingo_sdk::{CatalogEntry, ToolTraits, component::Tier};
    let look = ToolLook::new('¶', "read")
        .ascii('<')
        .target("file_path", Reads::Path);
    let mut entry = CatalogEntry {
        id: "Read".into(),
        label: "Read".into(),
        meta: json!({"look": look, "traits": ToolTraits::read_only()}),
    };
    assert_eq!(entry.look(), Some(look));
    assert_eq!(Tier::of(entry.traits().as_ref()), Tier::Lookup);
    entry.meta["traits"]["trusted"] = json!(false);
    assert_eq!(Tier::of(entry.traits().as_ref()), Tier::Change);
    entry.meta["traits"]["trusted"] = json!("true");
    assert!(entry.traits().is_none());
    assert_eq!(Tier::of(entry.traits().as_ref()), Tier::Change);
    entry.meta = Value::Null;
    assert!(entry.look().is_none());
    assert_eq!(Tier::of(entry.traits().as_ref()), Tier::Change);
    assert_eq!(Tier::of(Some(&ToolTraits::edit())), Tier::Change);
}
