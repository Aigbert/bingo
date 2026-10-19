//! World arrays ported from p4b-data.js and the p4d-crew.js roster.
use bingo_sdk::SessionSummary;
use serde_json::Value;

pub fn rows(name: &str) -> Vec<Value> {
    let world: Value = serde_json::from_str(include_str!("world.json")).unwrap();
    world[name].as_array().unwrap().clone()
}

pub fn roster(home: &SessionSummary) -> Vec<SessionSummary> {
    std::iter::once(home.clone())
        .chain(rows("roster").into_iter().enumerate().map(|(i, row)| {
            let mut summary = home.clone();
            let name = row["name"].as_str().unwrap();
            summary.id = bingo_sdk::SessionId::from_raw(name);
            summary.title = Some(name.into());
            summary.parent = Some(bingo_sdk::ParentLink {
                session: home.id.clone(),
                item: None,
            });
            summary.created_at = home
                .created_at
                .checked_add(jiff::Span::new().seconds(i as i64))
                .unwrap();
            summary
        }))
        .collect()
}
