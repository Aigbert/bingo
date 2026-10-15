//! One table from (row, key) to what the key does and what the footer says.
use bingo_sdk::{CatalogEntry, PluginState};
use bingo_services::mcp::{Auth, ServerRow, Status};
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Verb {
    Line(String),
    Trust,
    Review,
    Fold,
    Message(String),
    None,
}
/// A key a row answers to; `hint` is its footer words, absent when the key
/// only explains itself.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Binding {
    pub key: &'static str,
    pub hint: Option<String>,
    pub verb: Verb,
}
fn bind(key: &'static str, hint: impl Into<String>, verb: Verb) -> Binding {
    Binding {
        key,
        hint: Some(hint.into()),
        verb,
    }
}
fn quiet(key: &'static str, verb: Verb) -> Binding {
    Binding {
        key,
        hint: None,
        verb,
    }
}
/// The verb a pressed key names in a table, nothing for a key it lacks.
pub fn pressed(bindings: Vec<Binding>, key: &str) -> Verb {
    let key = if key == " " { "space" } else { key };
    bindings
        .into_iter()
        .find(|binding| binding.key == key)
        .map_or(Verb::None, |binding| binding.verb)
}
pub fn plugin(p: &CatalogEntry, key: &str) -> Verb {
    pressed(plugin_keys(p), key)
}
pub fn server(s: &ServerRow, reviewing: bool, key: &str) -> Verb {
    pressed(server_keys(s, reviewing), key)
}
pub fn builtins(expanded: bool) -> Vec<Binding> {
    vec![bind(
        "enter",
        if expanded { "fold" } else { "show them" },
        Verb::Fold,
    )]
}
pub fn plugin_keys(p: &CatalogEntry) -> Vec<Binding> {
    use crate::groups::{builtin, state};
    let Some(status) = state(p) else {
        return vec![quiet(
            "enter",
            Verb::Message(format!("{} is in a state this bingo cannot read", p.id)),
        )];
    };
    if matches!(status, PluginState::Waiting { .. }) {
        return vec![
            bind("enter", "trust it?", Verb::Trust),
            quiet("space", Verb::Message("⏎ to choose what it may do".into())),
        ];
    }
    let mut keys = vec![quiet("enter", Verb::Message(explain(&status).into()))];
    keys.push(switch(p, &status));
    if builtin(p) {
        keys.push(quiet(
            "r",
            Verb::Message(format!("{} is compiled in; nothing to reload", p.id)),
        ));
        keys.push(quiet(
            "t",
            Verb::Message("built in: compiled into bingo, always full".into()),
        ));
        return keys;
    }
    keys.push(if matches!(status, PluginState::Off { .. }) {
        quiet("r", Verb::Message("off; space turns it on".into()))
    } else {
        bind("r", "reload", Verb::Line("/plugins reload".into()))
    });
    keys.push(bind("t", "trust", Verb::Trust));
    keys
}
fn explain(status: &PluginState) -> &'static str {
    if matches!(status, PluginState::Failed { .. }) {
        "r reloads it; the error is on the card"
    } else {
        "space switches it; t changes its trust"
    }
}
fn switch(p: &CatalogEntry, status: &PluginState) -> Binding {
    if let Some(why) = p.meta["needed"].as_str() {
        return quiet("space", Verb::Message(format!("{} stays on: {why}", p.id)));
    }
    let on = match status {
        PluginState::Pending { now, .. } => !now,
        _ => status.is_on(),
    };
    bind(
        "space",
        if on { "turn off" } else { "turn on" },
        Verb::Line(format!(
            "/plugins {} {}",
            if on { "disable" } else { "enable" },
            p.id
        )),
    )
}
pub fn server_keys(s: &ServerRow, reviewing: bool) -> Vec<Binding> {
    let line = |verb: &str| Verb::Line(format!("/mcp {verb} {}", s.name));
    if let Status::Waiting { .. } = s.status {
        return waiting_keys(reviewing, line("approve"));
    }
    let wait = || Verb::Message("wait for the current connection or sign-in".into());
    let mut keys = vec![match s.status {
        Status::Connected { .. } => bind("enter", "its tools", line("tools")),
        Status::NeedsAuth { .. } => bind("enter", "sign in", line("login")),
        Status::Failed { .. } => bind("enter", "try again", line("reconnect")),
        Status::Disabled => bind("enter", "turn on", line("enable")),
        _ => quiet("enter", wait()),
    }];
    keys.push(if matches!(s.status, Status::Disabled) {
        bind("space", "turn on", line("enable"))
    } else {
        bind("space", "turn off", line("disable"))
    });
    keys.push(match s.status {
        Status::Connected { .. } | Status::Failed { .. } => {
            bind("r", "reconnect", line("reconnect"))
        }
        Status::NeedsAuth { .. } => quiet("r", Verb::Message("sign in first; ⏎ does it".into())),
        Status::Disabled => quiet("r", Verb::Message("off; space turns it on".into())),
        _ => quiet("r", wait()),
    });
    if matches!(s.auth, Some(Auth::SignedIn)) {
        keys.push(bind("o", "sign out", line("logout")));
    }
    keys
}
/// A server a repository brought runs nothing on one key: the first `⏎`
/// shows what would run, the second approves it.
fn waiting_keys(reviewing: bool, approve: Verb) -> Vec<Binding> {
    let review = Verb::Message("⏎ to review what it runs".into());
    if reviewing {
        return vec![
            bind("enter", "approve", approve),
            quiet("space", review.clone()),
            quiet("r", review),
        ];
    }
    vec![
        bind("enter", "review", Verb::Review),
        quiet("space", review.clone()),
        quiet("r", review),
    ]
}
