use super::*;
use serde_json::json;

fn entry(name: &str, family: &str) -> CatalogEntry {
    CatalogEntry {
        id: name.into(),
        label: name.into(),
        meta: json!({"name":name,"hint":format!("{name} hint"),"args":{"kind":"none"},"family":family}),
    }
}

/// Registered out of order on purpose: the list's order is its own.
fn catalogue() -> Vec<CatalogEntry> {
    let mut entries = [
        "provider",
        "experience",
        "rename",
        "rewind",
        "resume",
        "model",
        "status",
    ]
    .map(|name| entry(name, "session"))
    .to_vec();
    entries.insert(0, entry("release", "skill/project"));
    entries.insert(0, entry("review", "skill/project"));
    entries
}

fn labels(rows: &[Row]) -> Vec<&str> {
    rows.iter().map(|row| row.label.as_str()).collect()
}

#[test]
fn a_bare_slash_lists_the_known_order_then_the_skills() {
    let found = commands(&catalogue(), &BTreeMap::new(), "", PaletteHere::default());
    assert_eq!(
        labels(&found),
        [
            "model",
            "status",
            "resume",
            "rewind",
            "experience",
            "rename",
            "provider",
            "review",
            "release"
        ]
    );
    assert!(found.iter().all(|row| row.marks.is_empty()));
}

#[test]
fn prefix_then_run_then_letters_and_ties_keep_the_order() {
    let found = commands(&catalogue(), &BTreeMap::new(), "re", PaletteHere::default());
    assert_eq!(
        labels(&found),
        [
            "resume",
            "rewind",
            "rename",
            "review",
            "release",
            "experience",
            "provider"
        ]
    );
    assert_eq!(found[0].marks, vec![0, 1]);
    assert_eq!(found[5].marks, vec![4, 6]);
    let found = commands(&catalogue(), &BTreeMap::new(), "st", PaletteHere::default());
    assert_eq!(labels(&found), ["status"]);
}

#[test]
fn a_skill_carries_its_shelf_and_a_command_its_bound_key() {
    let bindings = BTreeMap::from([("alt+m".into(), "/model".into())]);
    let found = commands(&catalogue(), &bindings, "", PaletteHere::default());
    assert_eq!(found[0].now, "alt+m");
    assert_eq!(found[0].target, Target::Command("model".into()));
    assert_eq!(found[1].now, "");
    let review = found.iter().find(|row| row.label == "review").unwrap();
    assert_eq!(review.now, "project");
    assert_eq!(review.target, Target::Skill("review".into()));
}

#[test]
fn hidden_commands_stay_out() {
    let mut entries = catalogue();
    entries[2].meta["hidden"] = json!(true);
    assert!(
        commands(&entries, &BTreeMap::new(), "", PaletteHere::default())
            .iter()
            .all(|row| row.label != "provider")
    );
}

#[test]
fn an_alias_or_a_folded_name_finds_its_command_after_the_names() {
    let mut entries = vec![
        CatalogEntry {
            id: "exit".into(),
            label: "exit".into(),
            meta: json!({"name":"exit","aliases":["quit"],"hint":"leave","args":{"kind":"none"},"family":"surface"}),
        },
        entry("crew", "session"),
        entry("agents", "session"),
        entry("quiet", "session"),
    ];
    entries[2].meta["foldedInto"] = json!("crew");
    let found = commands(&entries, &BTreeMap::new(), "qui", PaletteHere::default());
    assert_eq!(labels(&found), ["quiet", "exit"]);
    assert!(found[1].marks.is_empty());
    let found = commands(&entries, &BTreeMap::new(), "agents", PaletteHere::default());
    assert_eq!(labels(&found), ["crew"]);
    assert_eq!(found[0].target, Target::Command("crew".into()));
}

#[test]
fn a_filtered_row_is_marked_where_the_query_matched() {
    let row = all_sessions();
    assert_eq!(filter(row.clone(), "ses").unwrap().marks, vec![4, 5, 6]);
    assert!(filter(row, "zz").is_none());
}

#[test]
fn room_commands_are_listed_in_an_open_room_and_skills_never_there() {
    let mut entries = catalogue();
    entries.push(entry("seat", ROOM_FAMILY));
    entries.push(entry("room", "rooms"));
    let listed = |driver, closed| {
        let here = PaletteHere { driver, closed };
        commands(&entries, &BTreeMap::new(), "", here)
            .iter()
            .map(|row| row.label.clone())
            .collect::<Vec<_>>()
    };
    let home = listed(Driver::Model, false);
    assert!(home.contains(&"review".to_string()), "{home:?}");
    assert!(!home.contains(&"seat".to_string()), "{home:?}");
    let open = listed(Driver::Log, false);
    assert!(open.contains(&"seat".to_string()), "{open:?}");
    assert!(open.contains(&"room".to_string()), "{open:?}");
    assert!(!open.contains(&"review".to_string()), "{open:?}");
    let closed = listed(Driver::Log, true);
    assert!(!closed.contains(&"seat".to_string()), "{closed:?}");
    assert!(closed.contains(&"room".to_string()), "{closed:?}");
}
