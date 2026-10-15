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
    let mut groups = vec![];
    if open == PluginsOpen::Mcp {
        groups.push(server_group(servers));
    }
    for (project, title, hint) in [
        (false, "plugins · yours", "~/.bingo/plugins"),
        (true, "plugins · this project", ".bingo/plugins"),
    ] {
        let rows = plugins
            .iter()
            .enumerate()
            .filter(|(_, p)| {
                !builtin(p)
                    && !p.id.starts_with("mcp:")
                    && (p.meta["origin"] == "project") == project
            })
            .map(|(i, p)| plugin_row(i, p, plugins))
            .collect::<Vec<_>>();
        if !rows.is_empty() {
            groups.push(Group {
                name: title,
                hint,
                rows,
            });
        }
    }
    groups.push(builtin_group(plugins, expanded));
    if open != PluginsOpen::Mcp {
        groups.push(server_group(servers));
    }
    groups
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

fn builtin_group(plugins: &[CatalogEntry], expanded: bool) -> Group {
    let builtins = plugins
        .iter()
        .enumerate()
        .filter(|(_, p)| builtin(p))
        .collect::<Vec<_>>();
    let off = builtins
        .iter()
        .filter(|(_, p)| !matches!(state(p), Some(PluginState::On)))
        .count();
    let under = builtins
        .iter()
        .filter(|(_, p)| crate::chain::drawn_over(p, plugins).is_some())
        .count();
    let tag = format!(
        "{}{}",
        if off == 0 {
            "all on".into()
        } else {
            format!("{off} off")
        },
        if under > 0 {
            format!(" · {under} drawn over")
        } else {
            String::new()
        }
    );
    let mut rows = if expanded {
        builtins
            .iter()
            .map(|(i, p)| plugin_row(*i, p, plugins))
            .collect()
    } else {
        vec![]
    };
    rows.push(Row {
        key: "builtins".into(),
        label: if expanded {
            "▴ fold".into()
        } else {
            format!("▸ {} built in", builtins.len())
        },
        glyph: "",
        tag,
        kind: Kind::Builtins,
    });
    Group {
        name: "plugins · built in",
        hint: "compiled into bingo",
        rows,
    }
}
