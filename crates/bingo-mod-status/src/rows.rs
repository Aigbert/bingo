use bingo_sdk::spent::{Price, spent};
use bingo_sdk::{CatalogEntry, component::StatusFacts};
use bingo_services::mcp::ServerRow;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Tone {
    Ok,
    Warn,
    Bad,
    Dim,
    Muted,
    Auto,
    Plan,
    Glow,
    Faint,
    Code,
}
#[derive(Clone, Debug, PartialEq)]
pub struct Span {
    pub text: String,
    pub tone: Tone,
}
#[derive(Clone, Debug, PartialEq)]
pub struct Row {
    pub label: String,
    pub value: Vec<Span>,
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
    state: &StatusFacts,
    plugins: &[CatalogEntry],
    servers: &[ServerRow],
    price: Option<Price>,
    glyphs: bingo_sdk::glyph::GlyphSet,
) -> Vec<Row> {
    let summary = &state.summary;
    let mut rows = vec![
        model(state, glyphs),
        permission(state, glyphs),
        context(state, glyphs),
    ];
    if let Some(cost) = spent(state.summary.usage, price).usd {
        let mut cost_row = row("cost", format!("${cost:.2}"), None);
        cost_row.value.push(Span {
            text: " this session".into(),
            tone: Tone::Dim,
        });
        rows.push(cost_row);
    }
    rows.push(row("directory", summary.cwd.clone(), None));
    rows.push(plugin_row(plugins, servers, glyphs));
    let rules = rules(state);
    let mut rule = row(
        "rules",
        if rules.is_empty() {
            "none yet".into()
        } else {
            rules.join(", ")
        },
        Some("permission"),
    );
    rule.value[0].tone = if rules.is_empty() {
        Tone::Dim
    } else {
        Tone::Code
    };
    rows.push(rule);
    rows
}

fn rules(state: &StatusFacts) -> Vec<&str> {
    state
        .config
        .policy()
        .and_then(|(_, p)| p["rules"].as_array())
        .into_iter()
        .flatten()
        .filter_map(|v| v.as_str())
        .collect()
}
fn model(state: &StatusFacts, glyphs: bingo_sdk::glyph::GlyphSet) -> Row {
    let summary = &state.summary;
    let mut row = row(
        "model",
        summary.model.clone().unwrap_or_default(),
        Some("model"),
    );
    row.value.push(Span {
        text: format!(" {}", bingo_sdk::Effort::word(summary.effort)),
        tone: Tone::Muted,
    });
    if let Some(provider) = &summary.provider {
        row.value.extend([
            Span {
                text: bingo_sdk::glyph::Glyph::Sep.spell(glyphs).into(),
                tone: Tone::Faint,
            },
            Span {
                text: provider.clone(),
                tone: Tone::Dim,
            },
        ]);
    }
    row
}
fn permission(state: &StatusFacts, glyphs: bingo_sdk::glyph::GlyphSet) -> Row {
    let mut row = row("mode", String::new(), Some("permission"));
    if let Some((current, modes)) = bingo_sdk::slot::described_config(&state.config)
        .into_iter()
        .filter(|placed| {
            state
                .config
                .policy()
                .is_some_and(|(id, _)| placed.plugin == id)
        })
        .find_map(|placed| match placed.slot {
            bingo_sdk::slot::Slot::Mode { current, modes } => Some((current, modes)),
            _ => None,
        })
    {
        row.value[0].text = bingo_sdk::slot::mode_label(&current, &modes).into();
        row.value[0].tone = match current.as_str() {
            "acceptEdits" => Tone::Auto,
            "plan" => Tone::Plan,
            "bypassPermissions" => Tone::Bad,
            _ => Tone::Ok,
        };
        if let Some(meaning) =
            bingo_sdk::slot::mode_at(&current, &modes).and_then(|at| modes[at].meaning.clone())
        {
            row.value.extend([
                Span {
                    text: bingo_sdk::glyph::Glyph::Sep.spell(glyphs).into(),
                    tone: Tone::Faint,
                },
                Span {
                    text: meaning,
                    tone: Tone::Dim,
                },
            ]);
        }
    } else if let Some(mode) = state.config.policy().and_then(|(_, p)| p["mode"].as_str()) {
        row.value[0].text = mode.into();
    }
    row
}

fn context(state: &StatusFacts, glyphs: bingo_sdk::glyph::GlyphSet) -> Row {
    let Some(c) = &state.context else {
        return row("context", String::new(), Some("compact"));
    };
    let cells = ((c.used as f64 / c.window.max(1) as f64 * 20.0).round() as usize).min(20);
    let ink = if c.percent() >= 80 {
        Tone::Warn
    } else {
        Tone::Glow
    };
    Row {
        label: "context".into(),
        value: vec![
            Span {
                text: bingo_sdk::glyph::Glyph::Heavy.spell(glyphs).repeat(cells),
                tone: ink,
            },
            Span {
                text: bingo_sdk::glyph::Glyph::Thin
                    .spell(glyphs)
                    .repeat(20 - cells),
                tone: Tone::Faint,
            },
            Span {
                text: format!("  {}% of {}", c.percent(), compact(c.window)),
                tone: if c.percent() >= 80 {
                    Tone::Warn
                } else {
                    Tone::Muted
                },
            },
        ],
        detail: None,
        command: Some("compact".into()),
    }
}

pub fn compact(n: u64) -> String {
    if n >= 1_000_000 {
        format!("{}M", n / 1_000_000)
    } else if n >= 1_000 {
        format!("{}k", n / 1_000)
    } else {
        n.to_string()
    }
}
fn plugin_row(
    plugins: &[CatalogEntry],
    servers: &[ServerRow],
    glyphs: bingo_sdk::glyph::GlyphSet,
) -> Row {
    use bingo_sdk::PluginState;
    let states = plugins
        .iter()
        .filter(|plugin| !plugin.id.starts_with("mcp:"))
        .map(|p| {
            (
                p,
                serde_json::from_value::<PluginState>(p.meta["state"].clone()).ok(),
            )
        })
        .collect::<Vec<_>>();
    let on = states
        .iter()
        .filter(|(_, s)| {
            matches!(
                s,
                Some(PluginState::On | PluginState::Pending { now: true, .. })
            )
        })
        .count();
    let on = on + servers.iter().filter(|server| server.forwardable()).count();
    let mut result = row("plugins", format!("{on} on"), Some("plugins"));
    if let Some((plugin, state)) = states.iter().find(|(_, s)| {
        matches!(
            s,
            Some(PluginState::Failed { .. } | PluginState::Waiting { .. }) | None
        )
    }) {
        let why = match state {
            Some(PluginState::Failed { why }) => why.as_str(),
            Some(PluginState::Waiting { .. }) => "waits for you",
            _ => "state unavailable",
        };
        result.value.push(Span {
            text: bingo_sdk::glyph::Glyph::Sep.spell(glyphs).into(),
            tone: Tone::Faint,
        });
        result.value.push(Span {
            text: format!("{} {why}", plugin.id),
            tone: Tone::Warn,
        });
    }
    if result.value.len() == 1
        && let Some(problem) = servers.iter().find_map(server_problem)
    {
        result.value.extend([
            Span {
                text: bingo_sdk::glyph::Glyph::Sep.spell(glyphs).into(),
                tone: Tone::Faint,
            },
            Span {
                text: problem,
                tone: Tone::Warn,
            },
        ]);
    }
    result
}

fn server_problem(server: &ServerRow) -> Option<String> {
    use bingo_services::mcp::Status;
    let why = match &server.status {
        Status::Failed { why } | Status::NeedsAuth { why } => why.as_str(),
        Status::Waiting { .. } => "waits for you",
        _ => return None,
    };
    Some(format!(
        "{} {}",
        server.name,
        why.lines().next().unwrap_or_default()
    ))
}

pub fn wake(
    pending: Option<&bingo_services::schedule::Pending>,
    now: jiff::Timestamp,
) -> Option<Row> {
    let pending = pending?;
    let at: jiff::Timestamp = pending.at.parse().ok()?;
    let seconds = (at.as_second() - now.as_second()).max(0);
    let remaining = if (60..3600).contains(&seconds) && seconds % 60 != 0 {
        format!("in {}m {}s", seconds / 60, seconds % 60)
    } else {
        bingo_sdk::until(now, at)
    };
    Some(Row {
        label: "wake".into(),
        value: vec![Span {
            text: format!("{remaining} · {}", pending.note),
            tone: Tone::Warn,
        }],
        detail: None,
        command: Some("wake off".into()),
    })
}
#[cfg(test)]
mod wake_tests {
    use super::*;
    #[test]
    fn none_future_and_past_due() {
        let now = jiff::Timestamp::UNIX_EPOCH;
        assert!(wake(None, now).is_none());
        for (at, expected) in [(240, "in 4m"), (220, "in 3m 40s"), (-1, "due now")] {
            let pending = bingo_services::schedule::Pending {
                at: jiff::Timestamp::from_second(at).unwrap().to_string(),
                note: "check".into(),
            };
            let row = wake(Some(&pending), now).unwrap();
            assert_eq!(row.value[0].text, format!("{expected} · check"));
            assert_eq!(row.value[0].tone, Tone::Warn);
            assert_eq!(row.command.as_deref(), Some("wake off"));
        }
    }
}
