use crate::spent::{Price, spent};
use bingo_sdk::{CatalogEntry, component::StatusProps};
use bingo_services::mcp::{ServerRow, Status};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Tone {
    Ok,
    Warn,
    Bad,
    Dim,
}
#[derive(Clone, Debug, PartialEq)]
pub struct Span {
    pub text: String,
    pub tone: Tone,
}
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Row {
    pub label: String,
    #[serde(skip)]
    pub value: Vec<Span>,
    #[serde(skip)]
    pub detail: Option<String>,
    pub command: Option<String>,
}
fn row(label: &str, text: String, command: Option<&str>) -> Row {
    Row {
        label: label.into(),
        value: vec![Span {
            text,
            tone: Tone::Ok,
        }],
        detail: None,
        command: command.map(str::to_owned),
    }
}
pub fn rows(
    state: &StatusProps,
    plugins: &[CatalogEntry],
    servers: &[ServerRow],
    price: Option<Price>,
) -> Vec<Row> {
    let summary = &state.summary;
    let mut rows = vec![
        row(
            "session",
            format!(
                "{} · {} · {}",
                summary.title.as_deref().unwrap_or("untitled"),
                summary.id,
                summary.cwd
            ),
            Some("rename"),
        ),
        row(
            "model",
            format!(
                "{} · {}",
                summary.model.as_deref().unwrap_or("unknown"),
                summary.provider.as_deref().unwrap_or("unknown")
            ),
            Some("model"),
        ),
        context(state),
        spent_row(state, price),
        permission(state),
    ];
    for rule in rules(state) {
        rows.push(row(
            &format!("allow {rule}"),
            rule.into(),
            Some(&format!("permission forget {rule}")),
        ));
    }
    rows.push(plugin_row(plugins));
    rows.push(server_row(servers));
    rows
}
fn rules(state: &StatusProps) -> Vec<&str> {
    state
        .config
        .plugins
        .get("bingo.permissions")
        .and_then(|p| p["rules"].as_array())
        .into_iter()
        .flatten()
        .filter_map(|v| v.as_str())
        .collect()
}
fn permission(state: &StatusProps) -> Row {
    let mode = state
        .config
        .plugins
        .get("bingo.permissions")
        .and_then(|p| p["mode"].as_str())
        .unwrap_or("default");
    row(
        "permission",
        format!("{mode} · {} rules this session", rules(state).len()),
        Some("permission"),
    )
}
fn spent_row(state: &StatusProps, price: Option<Price>) -> Row {
    let spent = spent(state.summary.usage, price);
    let cost = spent
        .usd
        .map(|n| format!("~${n:.2} at current rates"))
        .unwrap_or_else(|| "price unknown".into());
    let mut result = row(
        "spent",
        format!(
            "{} in · {} out · cache read {}% · {cost}",
            compact(spent.input),
            compact(spent.output),
            spent.cache_share
        ),
        None,
    );
    result.value[0].tone = Tone::Dim;
    result
}
fn context(state: &StatusProps) -> Row {
    let Some(c) = &state.context else {
        return row("context", "not measured yet".into(), Some("compact"));
    };
    let cells = ((u128::from(c.used) * 20) / u128::from(c.window.max(1))).min(20) as usize;
    let mut result = row(
        "context",
        format!(
            "{}{} {} of {} · {}% · compacts at {}",
            "▰".repeat(cells),
            "▱".repeat(20 - cells),
            compact(c.used),
            compact(c.window),
            c.percent(),
            compact(c.trigger)
        ),
        Some("compact"),
    );
    if c.used >= c.trigger && c.trigger > 0 {
        result.value[0].tone = Tone::Warn;
    }
    if let Some(parts) = &c.parts {
        result.detail = Some(format!(
            "about {}",
            parts
                .iter()
                .filter(|p| p.tokens > 0)
                .map(|p| format!("{} {}", p.name, compact(p.tokens)))
                .collect::<Vec<_>>()
                .join(" · ")
        ));
    }
    result
}
pub fn compact(n: u64) -> String {
    if n >= 1_000_000 {
        format!("{:.1}m", n as f64 / 1_000_000.0)
    } else if n >= 1_000 {
        format!("{}k", n / 1_000)
    } else {
        n.to_string()
    }
}
fn problem(row: &mut Row, names: &[&str], suffix: &str, tone: Tone) {
    let Some(first) = names.first() else {
        return;
    };
    let others = if names.len() > 1 {
        format!(" +{}", names.len() - 1)
    } else {
        String::new()
    };
    row.value.push(Span {
        text: format!(" · {first}{others} {suffix}"),
        tone,
    });
}
fn plugin_row(plugins: &[CatalogEntry]) -> Row {
    use bingo_sdk::PluginState;
    let (mut on, mut off) = (0, 0);
    let (mut failed, mut waiting, mut pending, mut unknown) = (vec![], vec![], vec![], vec![]);
    for plugin in plugins {
        let name = plugin.id.as_str();
        match serde_json::from_value::<PluginState>(plugin.meta["state"].clone()) {
            Ok(PluginState::On) => on += 1,
            Ok(PluginState::Off { .. }) => off += 1,
            Ok(PluginState::Failed { .. }) => failed.push(name),
            Ok(PluginState::Waiting { .. }) => waiting.push(name),
            Ok(PluginState::Registering | PluginState::Pending { .. }) => pending.push(name),
            Err(_) => unknown.push(name),
        }
    }
    let mut result = row("plugins", format!("{on} on · {off} off"), Some("plugins"));
    for (names, label, tone) in [
        (failed, "failed", Tone::Bad),
        (waiting, "from this repo waits for you", Tone::Warn),
        (pending, "pending", Tone::Warn),
        (unknown, "state unavailable", Tone::Warn),
    ] {
        problem(&mut result, &names, label, tone);
    }
    result
}
fn server_row(servers: &[ServerRow]) -> Row {
    let connected = servers
        .iter()
        .filter(|s| matches!(s.status, Status::Connected { .. }))
        .collect::<Vec<_>>();
    let mut result = row(
        "mcp",
        format!(
            "{} connected · {} tools",
            connected.len(),
            connected.iter().map(|s| s.tools()).sum::<usize>()
        ),
        Some("mcp"),
    );
    for (problem_state, tone) in [
        (Status::NeedsAuth { why: String::new() }, Tone::Warn),
        (Status::Failed { why: String::new() }, Tone::Bad),
        (Status::Connecting, Tone::Dim),
        (Status::SigningIn, Tone::Dim),
        (Status::Waiting { changed: false }, Tone::Warn),
    ] {
        let names = servers
            .iter()
            .filter(|s| std::mem::discriminant(&s.status) == std::mem::discriminant(&problem_state))
            .map(|s| s.name.as_str())
            .collect::<Vec<_>>();
        problem(&mut result, &names, problem_state.said(), tone);
    }
    result
}
