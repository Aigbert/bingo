use bingo_sdk::{CatalogEntry, PluginState};
use bingo_services::mcp::{Auth, ServerRow, Status};
#[derive(Clone, Debug)]
pub struct Fact {
    pub text: String,
    pub error: bool,
    pub aside: Option<String>,
}
fn line(text: impl Into<String>) -> Fact {
    Fact {
        text: text.into(),
        error: false,
        aside: None,
    }
}
fn title(name: &str, aside: &str) -> Fact {
    Fact {
        text: format!(" {name}"),
        error: false,
        aside: Some(aside.into()),
    }
}
fn fact(key: &str, value: impl std::fmt::Display) -> Fact {
    line(format!(" {key:8} {value}"))
}
/// An error line: indented inside the card, and cut at its edge by the
/// drawing rather than wrapped under the card's left margin.
fn error(text: &str) -> Fact {
    Fact {
        text: format!(" {text}"),
        error: true,
        aside: None,
    }
}
/// A failure's words after a blank line, one card line per line it has.
fn errors(why: &str) -> Vec<Fact> {
    std::iter::once(line(""))
        .chain(why.lines().map(error))
        .collect()
}
/// Where a path came from fits beside its key: its start and its end say
/// which one it is, so the middle goes.
const FROM: usize = 38;
pub fn middle(text: &str, width: usize) -> String {
    let count = text.chars().count();
    if count <= width {
        return text.into();
    }
    let keep = width.saturating_sub(1);
    let head = keep.div_ceil(2);
    let tail = keep - head;
    let start = text.chars().take(head);
    let end = text.chars().skip(count - tail);
    start.chain(std::iter::once('…')).chain(end).collect()
}
pub fn plugin_card(
    p: &CatalogEntry,
    all: &[CatalogEntry],
    trust: Option<&crate::trust::Choice>,
) -> Vec<Fact> {
    let mut rows = vec![title(crate::groups::name(p), kind(p)), line(""), line("")];
    rows.push(fact(
        "state",
        crate::groups::state(p).map_or("? unknown".into(), |state| plugin_state(&state)),
    ));
    rows.push(fact(
        "version",
        p.meta["version"].as_str().unwrap_or("unknown"),
    ));
    rows.push(fact("from", middle(&origin(p), FROM)));
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
    } else if p.meta["runtime"] == "module" {
        "js mod"
    } else {
        "plugin"
    }
}
fn origin(p: &CatalogEntry) -> String {
    if crate::groups::builtin(p) {
        "compiled into bingo".into()
    } else if p.meta["origin"] == "project" {
        format!(".bingo/plugins/{}", crate::groups::name(p))
    } else if p.meta["origin"] == "explicit" {
        "explicit plugin directory".into()
    } else if p.meta["origin"] == "user" {
        format!("~/.bingo/plugins/{}", crate::groups::name(p))
    } else {
        p.meta["from"]
            .as_str()
            .unwrap_or("configured plugin")
            .into()
    }
}
fn plugin_state(state: &PluginState) -> String {
    let reason = state.reason().unwrap_or_default();
    match state {
        PluginState::On => "● on".into(),
        PluginState::Off { .. } => "○ off".into(),
        PluginState::Failed { .. } => "✗ failed to load".into(),
        PluginState::Waiting { .. } => format!("◌ {reason}"),
        PluginState::Registering => "◌ loading …".into(),
        PluginState::Pending { now, .. } => {
            format!("{} → {reason}", if *now { "● on" } else { "○ off" })
        }
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
    if matches!(crate::groups::state(p), Some(PluginState::Waiting { .. })) {
        return "none yet".into();
    }
    if crate::groups::builtin(p) {
        return "full · compiled in".into();
    }
    let Some(tier) = p.meta["tier"].as_str() else {
        return "none yet".into();
    };
    let words = crate::trust::TIERS
        .iter()
        .find(|(name, _)| *name == tier)
        .map(|(_, words)| *words)
        .unwrap_or_default();
    format!("{tier} · {words}")
}
fn plugin_footer(p: &CatalogEntry) -> Vec<Fact> {
    let mut rows = match crate::groups::state(p) {
        Some(PluginState::Failed { why }) => errors(&why),
        _ => vec![],
    };
    rows.push(line(""));
    if let Some(why) = p.meta["needed"].as_str() {
        rows.push(line(format!(" needed: {why}")));
    } else if crate::groups::builtin(p) && !crate::chain::sites(p).is_empty() {
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
            matches!(crate::groups::state(p), Some(PluginState::Waiting { .. }))
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
            aside: None,
        }
    } else {
        line(" kept in ~/.bingo/settings.toml, never the repo")
    });
    rows
}
pub fn server_card(s: &ServerRow, reviewing: bool) -> Vec<Fact> {
    let mut rows = vec![title(&s.name, &format!("MCP · {}", s.transport.name()))];
    if reviewing {
        rows.extend(review(s));
    } else {
        rows.push(line(offered(&s.status)));
    }
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
        middle(s.origin.as_deref().unwrap_or("configured settings"), FROM),
    ));
    rows.push(fact("sign-in", sign_in(s)));
    if let Some(ms) = s.latency {
        rows.push(fact("latency", format!("{ms} ms")));
    }
    rows.push(fact("calls", "always ask; an MCP tool is never trusted"));
    if let Status::Failed { why } = &s.status {
        rows.extend(errors(why));
        if let Some(stderr) = &s.stderr {
            rows.push(error(&format!("its stderr: {stderr}")));
        }
    }
    rows
}
/// OAuth is named only where it is in play: a server that asked for a
/// sign-in or holds one. An HTTP server that never asked needs none, as the
/// `/mcp` table's dash says.
fn sign_in(s: &ServerRow) -> &str {
    let asked = matches!(s.status, Status::NeedsAuth { .. } | Status::SigningIn);
    match &s.auth {
        Some(Auth::SignedIn) => "OAuth · signed in",
        Some(Auth::Expired { .. }) => "OAuth · expired",
        Some(Auth::SignedOut) if asked => "OAuth · not signed in",
        _ => s.sign_in_from.as_deref().unwrap_or("none needed"),
    }
}
fn offered(status: &Status) -> String {
    match status {
        Status::Connected { tools } => format!(" {tools} tools for the model"),
        Status::NeedsAuth { .. } => " its tools appear once you sign in".into(),
        Status::Disabled => " its tools are not offered".into(),
        _ => " no tools until it runs".into(),
    }
}
/// What a yes would start, said first on the card so it fits any height,
/// before the second `⏎` gives it.
fn review(s: &ServerRow) -> Vec<Fact> {
    let what = if s.transport.name() == "http" {
        "dial"
    } else {
        "run"
    };
    vec![
        line(format!(
            " approving lets bingo {what}: {}",
            s.transport.runs()
        )),
        Fact {
            text: format!(" ⏎ again: approve {} for this checkout", s.name),
            error: true,
            aside: None,
        },
    ]
}
pub fn builtins_card(plugins: &[CatalogEntry], expanded: bool) -> Vec<Fact> {
    let builtins = plugins
        .iter()
        .filter(|p| crate::groups::builtin(p))
        .collect::<Vec<_>>();
    let needed = builtins
        .iter()
        .filter(|p| p.meta["needed"].is_string())
        .count();
    let fold = if expanded {
        " ⏎ fold them".to_string()
    } else {
        format!(" ⏎ show all {}", builtins.len())
    };
    [
        " built in",
        " compiled into bingo, each one a mod",
        "",
        " any of them can be drawn over: a mod in",
        " ~/.bingo/plugins that draws the same site",
        " is asked first and may call next",
        "",
        " turn one off and the plain drawing is used",
    ]
    .into_iter()
    .map(line)
    .chain([
        line(format!(" {needed} are needed and stay on")),
        line(""),
        line(fold),
    ])
    .collect()
}
