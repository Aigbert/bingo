use bingo_sdk::{BUILT_IN, CatalogEntry, PluginState, component::PluginsOpen};
use bingo_services::mcp::{ServerRow, Status};
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Kind {
    Plugin(usize),
    Server(usize),
    Builtins,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Row {
    pub key: String,
    pub label: String,
    pub glyph: &'static str,
    pub tag: String,
    pub kind: Kind,
}
#[derive(Clone, Debug)]
pub struct Group {
    pub name: &'static str,
    pub hint: &'static str,
    pub rows: Vec<Row>,
}
/// A plugin's state, or nothing when this build cannot read what the kernel
/// said: an unknown state is shown as unknown, never as one it is not.
pub fn state(plugin: &CatalogEntry) -> Option<PluginState> {
    serde_json::from_value(plugin.meta["state"].clone()).ok()
}
pub fn builtin(plugin: &CatalogEntry) -> bool {
    plugin.meta["from"] == BUILT_IN
}
pub fn name(plugin: &CatalogEntry) -> &str {
    plugin.id.strip_prefix("plugins:").unwrap_or(&plugin.id)
}
pub fn groups(
    open: PluginsOpen,
    plugins: &[CatalogEntry],
    servers: &[ServerRow],
    expanded: bool,
) -> Vec<Group> {
    let _ = (open, expanded);
    let mut rows = plugins
        .iter()
        .enumerate()
        .filter(|(_, p)| !p.id.starts_with("mcp:"))
        .map(|(i, p)| plugin_row(i, p, plugins))
        .collect::<Vec<_>>();
    rows.extend(server_group(servers).rows);
    rows.sort_by_key(|r| match r.kind {
        Kind::Plugin(i) => !matches!(
            state(&plugins[i]),
            Some(PluginState::Failed { .. } | PluginState::Waiting { .. })
        ),
        Kind::Server(i) => !matches!(
            servers[i].status,
            Status::Failed { .. } | Status::NeedsAuth { .. } | Status::Waiting { .. }
        ),
        Kind::Builtins => true,
    });
    vec![Group {
        name: "plugins",
        hint: "",
        rows,
    }]
}

fn plugin_row(index: usize, p: &CatalogEntry, plugins: &[CatalogEntry]) -> Row {
    let (glyph, tag) = match state(p) {
        Some(state) => (glyph(&state), tag(&state, p, plugins)),
        None => ("?", "unknown".into()),
    };
    Row {
        key: if p.meta["origin"] == "project" {
            format!("p:plugins:{}", name(p))
        } else {
            format!("p:{}", p.id)
        },
        label: name(p).into(),
        glyph,
        tag,
        kind: Kind::Plugin(index),
    }
}
fn glyph(state: &PluginState) -> &'static str {
    match state {
        PluginState::Failed { .. } => "✗",
        PluginState::Waiting { .. } | PluginState::Registering => "◌",
        _ if state.is_on() => "●",
        _ => "○",
    }
}
/// The row's words: the state's own reason where it has one worth a row.
fn tag(state: &PluginState, p: &CatalogEntry, plugins: &[CatalogEntry]) -> String {
    match state {
        PluginState::Waiting { .. } | PluginState::Pending { .. } => {
            state.reason().unwrap_or_default().into()
        }
        PluginState::Failed { .. } => "failed to load".into(),
        PluginState::Registering => "loading …".into(),
        PluginState::Off { .. } => "off".into(),
        PluginState::On if builtin(p) => builtin_tag(p, plugins),
        PluginState::On => p.meta["tier"].as_str().unwrap_or("on").into(),
    }
}
fn builtin_tag(p: &CatalogEntry, plugins: &[CatalogEntry]) -> String {
    if let Some(owner) = crate::chain::drawn_over(p, plugins) {
        format!("under {owner}")
    } else if p.meta["needed"].is_string() {
        "needed".into()
    } else {
        String::new()
    }
}
fn server_group(servers: &[ServerRow]) -> Group {
    Group {
        name: "MCP servers",
        hint: "their tools always ask",
        rows: servers
            .iter()
            .enumerate()
            .map(|(i, s)| {
                let (glyph, tag) = match &s.status {
                    Status::Connected { tools } => ("●", format!("{tools} tools")),
                    Status::NeedsAuth { .. } => ("○", "sign in".into()),
                    Status::Failed { .. } => ("✗", "failed".into()),
                    Status::Disabled => ("○", "off".into()),
                    Status::Waiting { changed } => (
                        "◌",
                        if *changed {
                            "changed since your yes"
                        } else {
                            "waits for your yes"
                        }
                        .into(),
                    ),
                    Status::Connecting => ("◌", "connecting …".into()),
                    Status::SigningIn => ("◌", "in the browser …".into()),
                };
                Row {
                    key: format!("m:{}", s.name),
                    label: s.name.clone(),
                    glyph,
                    tag: format!("{} · {tag}", s.transport.name()),
                    kind: Kind::Server(i),
                }
            })
            .collect(),
    }
}
/// The cursor's row only while it is still listed; a vanished row resets the
/// cursor to the first one and names nothing to act on.
pub fn current<'a>(groups: &'a [Group], cursor: &mut crate::Cursor) -> Option<&'a Row> {
    let listed = groups
        .iter()
        .flat_map(|group| &group.rows)
        .any(|row| row.key == cursor.selected);
    let row = selected(groups, cursor);
    if listed { row } else { None }
}
pub fn selected<'a>(groups: &'a [Group], cursor: &mut crate::Cursor) -> Option<&'a Row> {
    let row = groups
        .iter()
        .flat_map(|group| &group.rows)
        .find(|row| row.key == cursor.selected)
        .or_else(|| groups.iter().flat_map(|group| &group.rows).next())?;
    if cursor.selected != row.key {
        cursor.trust = None;
        cursor.reviewing = false;
        cursor.selected = row.key.clone();
    }
    Some(row)
}
