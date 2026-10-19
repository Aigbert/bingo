use bingo_sdk::SessionSummary;
use serde_json::{Value, json};

#[test]
fn ordered_teammates_keep_their_inks_across_roster_order_and_storage() {
    let cases: Vec<Value> = serde_json::from_str(include_str!("fixtures/faces.json")).unwrap();
    for case in cases {
        let mut rows: Vec<SessionSummary> = case["rows"]
            .as_array()
            .unwrap()
            .iter()
            .map(|row| {
                let room = row[3] == "room";
                serde_json::from_value(json!({
                    "id":row[0], "title":row[1], "cwd":"/workspace",
                    "createdAt":format!("2026-10-19T00:00:{:02}Z", row[2].as_u64().unwrap()),
                    "updatedAt":"2026-10-19T01:00:00Z",
                    "parent":if row[3] == "home" { Value::Null } else { json!({"session":row[4].as_str().unwrap_or("home")}) },
                    "driver":if room { "log" } else { "model" },
                    "key":if room { Some("rooms/home/design") } else { None },
                    "busy":row[3] == "live"
                }))
                .unwrap()
            })
            .collect();
        let mut home = case["openedFrom"].as_str().unwrap_or("home");
        while let Some(parent) = rows
            .iter()
            .find(|row| row.id.as_str() == home)
            .and_then(|row| row.parent.as_ref())
        {
            home = parent.session.as_str();
        }
        let home = home.to_owned();
        for (name, ink) in case["expected"].as_object().unwrap() {
            assert_eq!(
                bingo_collab::face::who(name, &home, &rows),
                ink.as_str().unwrap()
            );
            assert_eq!(
                bingo_collab::face::teammate(name, &home, &rows).is_some(),
                ink.as_str().unwrap().starts_with("crew"),
                "{name} is a teammate exactly when it wears a crew ink"
            );
        }
        rows.reverse();
        for (name, ink) in case["expected"].as_object().unwrap() {
            assert_eq!(
                bingo_collab::face::who(name, &home, &rows),
                ink.as_str().unwrap()
            );
        }
    }
}
