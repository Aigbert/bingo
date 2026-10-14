use crate::rows::{Group, Row};
use bingo_sdk::component::{self, PaletteProps};

pub fn append(groups: &mut Vec<Group>, props: &PaletteProps, query: &str) {
    if !query.is_empty() || props.open == component::PaletteOpen::Keys {
        let hints = rows(props, query);
        if let Some(keys) = groups.iter_mut().find(|group| group.name == "keys") {
            keys.rows.extend(hints);
        } else if !hints.is_empty() {
            let group = Group {
                name: "keys".into(),
                rows: hints,
            };
            if props.open == component::PaletteOpen::Keys {
                groups.insert(0, group);
            } else {
                groups.push(group);
            }
        }
    }
    order(groups, props);
}
fn order(groups: &mut [Group], props: &PaletteProps) {
    if let Some(keys) = groups.iter_mut().find(|group| group.name == "keys") {
        const FIRST: &[&str] = &[
            "shift+tab",
            "alt+m",
            "alt+t",
            "/",
            "?",
            "ctrl+r",
            "esc",
            "ctrl+o",
            "ctrl+t",
            "alt+enter",
            "ctrl+c",
        ];
        keys.rows.sort_by_key(|row| {
            FIRST
                .iter()
                .position(|key| *key == row.label)
                .unwrap_or(FIRST.len())
        });
        for row in &mut keys.rows {
            if row.label == "shift+tab" {
                row.now = props
                    .commands
                    .iter()
                    .find(|entry| entry.id == "permission")
                    .and_then(|entry| entry.meta["now"].as_str())
                    .unwrap_or_default()
                    .into();
            }
        }
    }
}

fn rows(props: &PaletteProps, query: &str) -> Vec<Row> {
    props
        .key_hints
        .iter()
        .filter(|(key, _)| !props.bindings.contains_key(*key))
        .map(|(key, hint)| Row {
            label: key.clone(),
            hint: hint.clone(),
            now: String::new(),
            marks: vec![],
            hint_marks: vec![],
            target: crate::rows::Target::Key(format!("@key:{key}")),
        })
        .filter_map(|row| crate::rows::filter(row, query))
        .collect()
}
