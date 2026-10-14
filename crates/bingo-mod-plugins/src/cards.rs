use bingo_sdk::{CatalogEntry, PluginState};
use bingo_services::mcp::{Auth, ServerRow, Status};
#[derive(Clone, Debug)]
pub struct Fact {
    pub text: String,
    pub error: bool,
}
fn line(text: impl Into<String>) -> Fact {
    Fact {
        text: text.into(),
        error: false,
    }
}
fn fact(key: &str, value: impl std::fmt::Display) -> Fact {
    line(format!(" {key:8} {value}"))
}
pub fn plugin_card(
    p: &CatalogEntry,
    all: &[CatalogEntry],
    trust: Option<&crate::trust::Choice>,
) -> Vec<Fact> {
    let mut rows = vec![
        line(format!(" {}", crate::groups::name(p))),
        fact("kind", kind(p)),
        line(""),
    ];
    rows.push(fact("state", plugin_state(&crate::groups::state(p))));
    rows.push(fact(
        "version",
        p.meta["version"].as_str().unwrap_or("unknown"),
    ));
    rows.push(fact("from", origin(p)));
    rows.extend(contributions(p));
    for site in crate::chain::sites(p) {
        rows.push(fact("draws", &site));
        rows.push(fact("", crate::chain::chain_of(&site, all).join(" ▸ ")));
    }
    rows.push(fact("trust", granted(p)));
    if let Some(trust) = trust {
        rows.extend(trust_card(p, trust));
    } else {
        rows.extend(plugin_footer(p));
    }
    rows
}
fn kind(p: &CatalogEntry) -> &str {
    if crate::groups::builtin(p) {
        "compiled in"
    } else if p.meta["runtime"] == "process" {
        "process"
    } else {
        "js mod"
    }
}
fn origin(p: &CatalogEntry) -> String {
    if crate::groups::builtin(p) {
        "compiled into bingo".into()
    } else if p.meta["origin"] == "project" {
        format!(".bingo/plugins/{}", crate::groups::name(p))
    } else if p.meta["origin"] == "explicit" {
        "explicit plugin directory".into()
    } else {
        format!("~/.bingo/plugins/{}", crate::groups::name(p))
    }
}
fn plugin_state(state: &PluginState) -> String {
    match state {
        PluginState::On => "● on".into(),
        PluginState::Off { .. } => "○ off".into(),
        PluginState::Failed { .. } => "✗ failed to load".into(),
        PluginState::Waiting { changed: true } => "◌ changed since your yes".into(),
        PluginState::Waiting { .. } => "◌ waiting for your yes".into(),
        PluginState::Registering => "◌ loading …".into(),
        PluginState::Pending { now, at } => format!(
            "{} → {} {}",
            if *now { "● on" } else { "○ off" },
            if *now { "off" } else { "on" },
            match at {
                bingo_sdk::plugin::NextStart::AfterTurn => "after this turn",
                _ => "at next start",
            }
        ),
    }
}
fn contributions(p: &CatalogEntry) -> Vec<Fact> {
    let mut rows = vec![];
    let commands = p.meta["commands"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|v| serde_json::from_value::<bingo_sdk::CommandSpec>(v.clone()).ok())
        .collect::<Vec<_>>();
    let adds = commands
        .iter()
        .flat_map(|s| std::iter::once(format!("/{}", s.name)).chain(s.keys.clone()))
        .collect::<Vec<_>>();
    if !adds.is_empty() {
        rows.push(fact("adds", adds.join(" · ")));
    }
    let provides = p.meta["tools"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|v| v.as_str())
        .chain(
            p.meta["provides"]
                .as_array()
                .into_iter()
                .flatten()
                .filter_map(|v| v.as_str())
                .filter(|v| v.starts_with("tool:") || v.starts_with("tools:")),
        )
        .collect::<Vec<_>>();
    if !provides.is_empty() {
        rows.push(fact("tools", provides.join(", ")));
    }
    let hooks = crate::chain::hooks(p);
    if !hooks.is_empty() {
        rows.push(fact("hooks", hooks.join(", ")));
    }
    rows
}
fn granted(p: &CatalogEntry) -> String {
    if matches!(crate::groups::state(p), PluginState::Waiting { .. }) {
        return "none yet".into();
    }
    if crate::groups::builtin(p) {
        return "full · compiled in".into();
    }
    let tier = p.meta["tier"].as_str().unwrap_or("tighten");
    let words = crate::trust::TIERS
        .iter()
        .find(|(name, _)| *name == tier)
        .map(|(_, words)| *words)
        .unwrap_or_default();
    format!("{tier} · {words}")
}
fn plugin_footer(p: &CatalogEntry) -> Vec<Fact> {
    let mut rows = vec![line("")];
    if let PluginState::Failed { why } = crate::groups::state(p) {
        rows.push(Fact {
            text: why,
            error: true,
        });
    }
    if let Some(why) = p.meta["needed"].as_str() {
        rows.push(line(format!(" needed: {why}")));
    } else if p.id.starts_with("bingo.mod.") {
        rows.push(line(" off: the plain drawing is used"));
    }
    rows
}
fn trust_card(p: &CatalogEntry, choice: &crate::trust::Choice) -> Vec<Fact> {
    let mut rows = vec![
        line(""),
        line(format!(" trust {} at", crate::groups::name(p))),
    ];
    for (i, (tier, words)) in crate::trust::TIERS
        .iter()
        .chain(
            matches!(crate::groups::state(p), PluginState::Waiting { .. })
                .then_some(&("not now", "leave it off")),
        )
        .enumerate()
    {
        rows.push(line(format!(
            " {} {tier:9} {words}",
            if i == choice.selected { "❯" } else { " " }
        )));
    }
    rows.push(if choice.confirm {
        Fact {
            text: format!(
                " ⏎ again: {} could run tools unasked",
                crate::groups::name(p)
            ),
            error: true,
        }
    } else {
        line(" kept in ~/.bingo/settings.toml, never the repo")
    });
    rows
}
pub fn server_card(s: &ServerRow) -> Vec<Fact> {
    let mut rows = vec![line(format!(" {}    MCP · {}", s.name, s.transport.name()))];
    rows.push(line(match s.status {
        Status::Connected { tools } => format!(" {tools} tools for the model"),
        Status::NeedsAuth { .. } => " its tools appear once you sign in".into(),
        Status::Disabled => " its tools are not offered".into(),
        _ => " no tools until it runs".into(),
    }));
    rows.push(line(""));
    rows.push(fact(
        "state",
        match &s.status {
            Status::Connected { .. } => "● connected",
            Status::Disabled => "○ off",
            Status::Failed { .. } => "✗ not running",
            Status::NeedsAuth { .. } => "○ needs sign-in",
            Status::Waiting { .. } => "◌ waiting for your yes",
            Status::Connecting => "◌ connecting …",
            Status::SigningIn => "◌ waiting for the browser",
        },
    ));
    rows.push(fact(
        if s.transport.name() == "http" {
            "address"
        } else {
            "runs"
        },
        s.transport.runs(),
    ));
    rows.push(fact(
        "from",
        s.origin.as_deref().unwrap_or("configured settings"),
    ));
    rows.push(fact(
        "sign-in",
        match &s.auth {
            Some(Auth::SignedIn) => "OAuth · signed in",
            Some(Auth::SignedOut) => "OAuth · not signed in",
            Some(Auth::Expired { .. }) => "OAuth · expired",
            None => s.sign_in_from.as_deref().unwrap_or("none needed"),
        },
    ));
    if let Some(ms) = s.latency {
        rows.push(fact("latency", format!("{ms} ms")));
    }
    rows.push(fact("calls", "always ask; an MCP tool is never trusted"));
    if let Status::Failed { why } = &s.status {
        rows.push(line(""));
        rows.push(Fact {
            text: why.clone(),
            error: true,
        });
    }
    rows
}
pub fn builtins_card(expanded: bool) -> Vec<Fact> {
    [
        " built in",
        " compiled into bingo, each one a mod",
        "",
        " any of them can be drawn over: a mod in",
        " ~/.bingo/plugins that draws the same site",
        " is asked first and may call next",
        "",
        " turn one off and the plain drawing is used",
        if expanded {
            " ⏎ fold them"
        } else {
            " ⏎ show them"
        },
    ]
    .into_iter()
    .map(line)
    .collect()
}
