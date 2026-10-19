//! The world a sheet scene starts from: the prototype's arrays as the
//! catalogues, services and sessions the owners read through the host.
use super::kernel::Kernel;
use crate::test_support::ember::{CLOCK_ORIGIN_MS, world::rows};
use bingo_sdk::*;
use serde_json::json;

/// The host answers through the kernel: what an owner opens, signals or
/// draws reaches the root session and the surface in order.
pub(super) fn script(kernel: &Kernel, switched: &Switched) {
    let host = kernel.host();
    let k = kernel.clone();
    host.on_open(move |selector, _, _| {
        let attached = match selector {
            SessionSelector::ById { id } if id != k.root().summary.id => past(id),
            _ => k.attach(),
        };
        async move { Ok(attached) }
    });
    host.on_sessions(|_| async { Ok(sessions()) });
    let k = kernel.clone();
    host.on_signal(move |_, plugin, kind, payload| {
        k.broadcast(Event::Signal {
            generation: None,
            plugin,
            kind,
            payload,
        });
        async { Ok(()) }
    });
    let k = kernel.clone();
    host.on_extend(move |_, plugin, kind, payload| {
        k.broadcast(Event::Extension {
            plugin,
            kind,
            payload,
        });
        async { Ok(()) }
    });
    let k = kernel.clone();
    host.on_rewind(move |_, to_turn| {
        let rewound = k.rewind(to_turn);
        async move { rewound }
    });
    let k = kernel.clone();
    host.on_render(move |_, request| {
        let k = k.clone();
        async move { k.render(request).await }
    });
    let (k, switched) = (kernel.clone(), switched.clone());
    host.on_catalog(move |kind| {
        let entries = catalog(&k, &switched, kind);
        async move { Ok(Catalog { entries, kind }) }
    });
}

fn catalog(kernel: &Kernel, switched: &Switched, kind: CatalogKind) -> Vec<CatalogEntry> {
    match kind {
        CatalogKind::Plugins => plugins(switched),
        CatalogKind::Commands => kernel.catalogue(),
        CatalogKind::Tools => crate::test_support::ember::story::catalogue(),
        CatalogKind::Models => models(),
        CatalogKind::Providers => providers(),
        _ => vec![],
    }
}

/// The prototype's session spent $0.30 over its two turns (`costOf`) and
/// fills 21% of its million-token window.
pub(super) const SPENT: Usage = Usage {
    input_tokens: 100_000,
    output_tokens: 0,
    cache_read_tokens: 0,
    cache_write_tokens: 0,
    reasoning_tokens: 0,
};
const PRICE_PER_MILLION: f64 = 3.0;
pub(super) const CONTEXT: ContextUsage = ContextUsage {
    parts: None,
    used: 210_000,
    window: 1_000_000,
    trigger: 1_000_000,
};

fn models() -> Vec<CatalogEntry> {
    rows("models")
        .into_iter()
        .map(|row| {
            let id = row["id"].as_str().unwrap();
            let provider = row["prov"].as_str().unwrap();
            let ctx = row["ctx"].as_str().unwrap();
            let scale = if ctx.ends_with('M') { 1_000_000 } else { 1000 };
            let context = ctx[..ctx.len() - 1].parse::<u64>().unwrap() * scale;
            let mut meta = json!({"provider":provider,"context":context,"images":row["img"],"efforts":row["effs"]});
            if id == "opus-5.5" {
                meta["price"] = json!({ "input": PRICE_PER_MILLION });
            }
            CatalogEntry {
                id: format!("{provider}/{id}"),
                label: id.into(),
                meta,
            }
        })
        .collect()
}

/// The plugins the person switched against the world's own settings.
#[derive(Clone, Default)]
pub(super) struct Switched(std::sync::Arc<std::sync::Mutex<std::collections::BTreeSet<String>>>);

impl Switched {
    pub(super) fn flip(&self, id: &str) {
        let mut flipped = self.0.lock().unwrap();
        if !flipped.remove(id) {
            flipped.insert(id.to_owned());
        }
    }

    fn flipped(&self, id: &str) -> bool {
        self.0.lock().unwrap().contains(id)
    }
}

/// Every plugin of the prototype's list, as the host catalogues them.
fn plugins(switched: &Switched) -> Vec<CatalogEntry> {
    rows("plugins")
        .into_iter()
        .map(|row| {
            let name = row["n"].as_str().unwrap();
            let id = if row["kind"] == "mcp" {
                format!("mcp:{name}")
            } else {
                name.into()
            };
            let on = row["on"].as_bool().unwrap() != switched.flipped(&id);
            let from = if row["kind"] == "built in" {
                BUILT_IN
            } else {
                "user"
            };
            let state = if on {
                json!({"kind":"on"})
            } else {
                json!({"kind":"off","why":"switched off"})
            };
            CatalogEntry {
                id,
                label: name.into(),
                meta: json!({"from":from,"description":row["d"],"state":state}),
            }
        })
        .collect()
}

/// The MCP servers the prototype lists: github's token has expired and
/// linear is switched off.
pub(super) fn servers() -> Vec<bingo_services::mcp::ServerRow> {
    serde_json::from_value(json!([
        {"name":"github","transport":{"type":"http","url":"https://example.test"},
         "status":{"kind":"needsAuth","why":"token expired"}},
        {"name":"linear","transport":{"type":"http","url":"https://example.test"},
         "status":{"kind":"disabled"}}
    ]))
    .unwrap()
}

/// Every provider the models name, signed in and answering.
fn providers() -> Vec<CatalogEntry> {
    let mut names: Vec<String> = rows("models")
        .iter()
        .map(|row| row["prov"].as_str().unwrap().to_owned())
        .collect();
    names.dedup();
    names
        .into_iter()
        .map(|name| CatalogEntry {
            id: name.clone(),
            label: name,
            meta: json!({"auth":{"kind":"ok"},"reach":{"kind":"ok"}}),
        })
        .collect()
}

/// The prototype's task list, as bingo-tasks journals it on the session.
pub(super) fn tasks() -> Event {
    let tasks: Vec<_> = rows("tasks")
        .into_iter()
        .map(|row| json!({"id":row["id"],"subject":row["t"],"status":row["st"],"owner":row["who"]}))
        .collect();
    Event::Extension {
        plugin: "bingo.tasks".into(),
        kind: "tasks".into(),
        payload: json!({ "tasks": tasks }),
    }
}

/// The prototype's session directory, aged in the whole minutes the
/// directory reads its clock in.
fn sessions() -> Vec<SessionSummary> {
    let now = jiff::Timestamp::from_second(CLOCK_ORIGIN_MS / 60_000 * 60).unwrap();
    rows("sessions")
        .into_iter()
        .enumerate()
        .map(|(i, row)| {
            let mut summary = crate::test_support::state().summary;
            summary.id = SessionId::from_raw(format!("session-{i}"));
            summary.title = Some(row["t"].as_str().unwrap().into());
            summary.cwd = row["dir"].as_str().unwrap().into();
            summary.asking = row["st"] == "asking";
            summary.busy = row["st"] == "working";
            summary.updated_at = when(row["when"].as_str().unwrap(), now);
            summary
        })
        .collect()
}

/// `12m` and `2h` ago, or a day of a year the scene's clock has long left.
fn when(word: &str, now: jiff::Timestamp) -> jiff::Timestamp {
    if let Some(minutes) = word.strip_suffix('m') {
        return now - jiff::SignedDuration::from_mins(minutes.parse().unwrap());
    }
    if let Some(hours) = word.strip_suffix('h') {
        return now - jiff::SignedDuration::from_hours(hours.parse().unwrap());
    }
    let (month, day) = word.split_once(' ').unwrap();
    let month = match month {
        "Sep" => 9,
        "Aug" => 8,
        _ => panic!("unknown month {month}"),
    };
    format!(
        "1969-{month:02}-{:02}T00:00:00Z",
        day.parse::<u8>().unwrap()
    )
    .parse()
    .unwrap()
}

/// A listed session, opened: the prototype's past conversation under its
/// title, finished long before the scene began.
fn past(id: SessionId) -> Attachment {
    let title = sessions()
        .into_iter()
        .find(|summary| summary.id == id)
        .and_then(|summary| summary.title)
        .unwrap();
    let mut state = crate::test_support::state();
    let mut summary = state.summary.clone();
    summary.title = Some(title.clone());
    state.apply(&crate::test_support::frame(
        state.seq.0 + 1,
        Event::SessionUpdated { summary },
    ));
    let start = jiff::Timestamp::from_millisecond(CLOCK_ORIGIN_MS).unwrap()
        - jiff::SignedDuration::from_secs(10_000);
    crate::test_support::ember::story::script(&mut state, "pastScript", &title, start, 1300);
    Attachment {
        session: state.summary.id.clone(),
        snapshot: state,
        history: None,
        events: Box::pin(futures::stream::pending()),
        handle: SessionHandle(std::sync::Arc::new(crate::doubles::TestSession::default())),
    }
}
