use bingo_sdk::{
    CatalogEntry, CommandSpec,
    component::{PaletteOpen, PaletteSession},
};
use std::collections::BTreeMap;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Target {
    Command(String),
    Key(String),
    Session(String),
    View(String),
    Inert,
    Edit(String),
    Run(String),
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Row {
    pub label: String,
    pub hint: String,
    pub alert: Option<String>,
    pub now: String,
    pub marks: Vec<usize>,
    pub hint_marks: Vec<usize>,
    pub target: Target,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Group {
    pub name: String,
    pub rows: Vec<Row>,
}

pub fn matched(text: &str, query: &str) -> Option<Vec<usize>> {
    let mut wanted = query.chars().flat_map(char::to_lowercase).peekable();
    let mut indices = Vec::new();
    for (index, c) in text.chars().enumerate() {
        if c.to_lowercase().next().as_ref() == wanted.peek() {
            wanted.next();
            indices.push(index);
        }
    }
    wanted.peek().is_none().then_some(indices)
}

pub fn rows(
    opened: PaletteOpen,
    query: &str,
    commands: &[CatalogEntry],
    bindings: &BTreeMap<String, String>,
    sessions: &[PaletteSession],
    recent: &[String],
    current: &str,
) -> Vec<Group> {
    let mut groups = vec![
        Group {
            name: "commands".into(),
            rows: command_rows(commands, query),
        },
        Group {
            name: "sessions".into(),
            rows: sessions.iter().map(|s| session(s, current)).collect(),
        },
        Group {
            name: "keys".into(),
            rows: bindings
                .iter()
                .filter_map(|(key, line)| binding(key, line, commands))
                .collect(),
        },
    ];
    let first = match opened {
        PaletteOpen::Commands => "commands",
        PaletteOpen::Keys => "keys",
        PaletteOpen::Sessions => "sessions",
    };
    groups.sort_by_key(|group| group.name != first);
    if query.is_empty() {
        groups.retain(|group| group.name == first);
        if opened == PaletteOpen::Commands {
            recent_first(&mut groups, recent);
        }
    } else {
        for group in &mut groups {
            group.rows = group
                .rows
                .drain(..)
                .filter_map(|row| filter(row, query))
                .collect();
            if group.name == "commands" {
                group.rows.sort_by_key(|row| {
                    (
                        row.label.trim_start_matches('/') != query,
                        row.marks.is_empty(),
                    )
                });
            }
        }
    }
    groups.retain(|group| !group.rows.is_empty());
    groups
}
fn command_rows(entries: &[CatalogEntry], query: &str) -> Vec<Row> {
    const FIRST: &[&str] = &[
        "model",
        "think",
        "permission",
        "rename",
        "compact",
        "resume",
        "login",
        "logout",
        "status",
        "plugins",
        "mcp",
        "agents",
        "memory",
        "help",
    ];
    let mut rows = entries
        .iter()
        .filter_map(|entry| command(entry, query))
        .collect::<Vec<_>>();
    rows.sort_by_key(|row| {
        FIRST
            .iter()
            .position(|name| *name == row.label.trim_start_matches('/'))
            .unwrap_or(FIRST.len())
    });
    rows
}
fn recent_first(groups: &mut Vec<Group>, recent: &[String]) {
    let Some(commands) = groups.first_mut() else {
        return;
    };
    let mut rows = Vec::new();
    for name in recent {
        if let Some(at) = commands
            .rows
            .iter()
            .position(|row| row.target == Target::Command(name.clone()))
        {
            rows.push(commands.rows.remove(at));
        }
    }
    if !rows.is_empty() {
        groups.insert(
            0,
            Group {
                name: "recent".into(),
                rows,
            },
        );
    }
}
pub(crate) fn filter(mut row: Row, query: &str) -> Option<Row> {
    if let Some(marks) = matched(&row.label, query) {
        row.marks = marks;
        Some(row)
    } else if row.hint.to_lowercase().contains(&query.to_lowercase()) {
        matched(&row.hint, query).map(|marks| {
            row.hint_marks = marks;
            row
        })
    } else {
        None
    }
}
fn command(entry: &CatalogEntry, query: &str) -> Option<Row> {
    let spec: CommandSpec = serde_json::from_value(entry.meta.clone()).ok()?;
    (!spec.hidden).then(|| Row {
        label: format!("/{}", spelling(&spec, query)),
        hint: spec.hint,
        alert: None,
        now: entry.meta["now"].as_str().unwrap_or_default().into(),
        marks: vec![],
        hint_marks: vec![],
        target: Target::Command(spec.name),
    })
}
fn spelling<'a>(spec: &'a CommandSpec, query: &str) -> &'a str {
    let names =
        || std::iter::once(spec.name.as_str()).chain(spec.aliases.iter().map(String::as_str));
    names()
        .find(|name| *name == query)
        .or_else(|| names().find(|name| matched(name, query).is_some()))
        .unwrap_or(&spec.name)
}
fn binding(key: &str, line: &str, commands: &[CatalogEntry]) -> Option<Row> {
    let name = line
        .trim_start_matches('/')
        .split_whitespace()
        .next()
        .unwrap_or_default();
    let entry = commands.iter().find(|entry| entry.id == name)?;
    if entry.meta["hidden"].as_bool().unwrap_or(false) {
        return None;
    }
    Some(Row {
        label: key.into(),
        hint: entry.meta["hint"].as_str().unwrap_or(line).into(),
        alert: None,
        now: entry.meta["now"].as_str().unwrap_or_default().into(),
        marks: vec![],
        hint_marks: vec![],
        target: Target::Key(line.into()),
    })
}
fn session(summary: &PaletteSession, current: &str) -> Row {
    Row {
        label: summary.label.clone(),
        hint: String::new(),
        alert: summary.attention.clone(),
        now: if summary.id.as_str() == current {
            "this one".into()
        } else {
            summary.detail.clone()
        },
        marks: vec![],
        hint_marks: vec![],
        target: Target::Session(summary.id.to_string()),
    }
}
pub fn conversation(sessions: &[PaletteSession], query: &str, current: &str) -> Vec<Row> {
    sessions
        .iter()
        .map(|summary| {
            let mut row = session(summary, current);
            row.target = Target::View(summary.id.to_string());
            row.hint = summary.detail.clone();
            if summary.id.as_str() != current {
                row.now.clear();
            }
            row
        })
        .filter_map(|row| filter(row, query))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    fn commands() -> Vec<CatalogEntry> {
        ["model","more","hidden"].map(|name| CatalogEntry { id:name.into(), label:name.into(), meta:json!({"name":name,"hint":"choose model","args":{"kind":"none"},"family":"session","hidden":name=="hidden"}) }).to_vec()
    }
    #[test]
    fn openers_search_all_groups_but_empty_queries_keep_their_own() {
        let commands = commands();
        let keys = BTreeMap::from([("alt+m".into(), "/model".into())]);
        for open in [
            PaletteOpen::Commands,
            PaletteOpen::Keys,
            PaletteOpen::Sessions,
        ] {
            let found = rows(open, "mo", &commands, &keys, &[], &[], "s");
            assert_eq!(found.iter().map(|g| g.rows.len()).sum::<usize>(), 3);
            if open == PaletteOpen::Keys {
                assert_eq!(found[0].name, "keys");
            }
        }
        assert!(rows(PaletteOpen::Sessions, "", &commands, &keys, &[], &[], "s").is_empty());
        let found = rows(
            PaletteOpen::Commands,
            "",
            &commands,
            &keys,
            &[],
            &["model".into()],
            "s",
        );
        assert_eq!(found[0].name, "recent");
        assert_eq!(found[1].rows.len(), 1);
        assert_eq!(matched("/model", "md"), Some(vec![1, 3]));
        assert_eq!(matched("model", "zz"), None);
    }
    #[test]
    fn hidden_bindings_stay_hidden_and_hint_matches_are_marked() {
        let bindings = BTreeMap::from([("alt+h".into(), "/hidden".into())]);
        assert!(rows(PaletteOpen::Keys, "", &commands(), &bindings, &[], &[], "s").is_empty());
        let found = rows(
            PaletteOpen::Commands,
            "choose",
            &commands(),
            &bindings,
            &[],
            &[],
            "s",
        );
        assert_eq!(found[0].rows.len(), 2);
        assert_eq!(found[0].rows[0].hint_marks, vec![0, 1, 2, 3, 4, 5]);
    }
    #[test]
    fn an_exact_command_cannot_be_shadowed_by_a_description() {
        let commands = [("agents", "the sub-agents this session started, or stop or dismiss one"), ("board", "show the board")].map(|(name, hint)| CatalogEntry { id:name.into(), label:name.into(), meta:serde_json::json!({"name":name,"hint":hint,"args":{"kind":"none"},"family":"fixture"}) });
        let found = rows(
            PaletteOpen::Commands,
            "board",
            &commands,
            &BTreeMap::new(),
            &[],
            &[],
            "s",
        );
        assert_eq!(found[0].rows.len(), 1);
        assert_eq!(found[0].rows[0].target, Target::Command("board".into()));
        let mut commands = commands;
        commands[0].meta["hint"] = serde_json::json!("show the board");
        let found = rows(
            PaletteOpen::Commands,
            "board",
            &commands,
            &BTreeMap::new(),
            &[],
            &[],
            "s",
        );
        assert_eq!(found[0].rows.len(), 2);
        assert_eq!(found[0].rows[0].target, Target::Command("board".into()));
    }
    #[test]
    fn common_controls_lead_extensions_regardless_of_registration_order() {
        let mut commands = commands();
        commands.insert(0, CatalogEntry { id:"permission".into(), label:"permission".into(), meta:serde_json::json!({"name":"permission","hint":"change mode","args":{"kind":"none"},"family":"session"}) });
        for query in ["", "mo"] {
            let found = rows(
                PaletteOpen::Commands,
                query,
                &commands,
                &BTreeMap::new(),
                &[],
                &[],
                "s",
            );
            assert_eq!(found[0].rows[0].target, Target::Command("model".into()));
        }
    }
}

#[cfg(test)]
mod review_tests {
    use super::*;
    #[test]
    fn aliases_are_searchable_and_keep_the_canonical_target() {
        let entry = CatalogEntry {
            id: "exit".into(),
            label: "exit".into(),
            meta: serde_json::json!({"name":"exit","aliases":["quit"],"hint":"leave","args":{"kind":"none"},"family":"surface"}),
        };
        let rows = rows(
            PaletteOpen::Commands,
            "quit",
            &[entry],
            &BTreeMap::new(),
            &[],
            &[],
            "s",
        );
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].rows[0].label, "/quit");
        assert_eq!(rows[0].rows[0].target, Target::Command("exit".into()));
    }
}

pub(crate) fn all_sessions() -> Row {
    Row {
        label: "All sessions…".into(),
        hint: String::new(),
        alert: None,
        now: String::new(),
        marks: vec![],
        hint_marks: vec![],
        target: Target::Run("/sessions".into()),
    }
}
