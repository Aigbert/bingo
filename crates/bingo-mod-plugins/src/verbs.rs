use bingo_sdk::{CatalogEntry, PluginState};
use bingo_services::mcp::{Auth, ServerRow, Status};
#[derive(Debug, PartialEq, Eq)]
pub enum Verb {
    Line(String),
    Trust,
    Fold,
    Message(String),
    None,
}
pub fn plugin(p: &CatalogEntry, key: &str) -> Verb {
    use crate::groups::{builtin, state};
    let status = state(p);
    match key {
        "space" | " " | "enter" if matches!(status, PluginState::Waiting { .. }) => Verb::Trust,
        "space" | " " => {
            if let Some(why) = p.meta["needed"].as_str() {
                return Verb::Message(format!("{} stays on: {why}", p.id));
            }
            let wish = match status {
                PluginState::Pending { now, .. } => !now,
                _ => status.is_on(),
            };
            Verb::Line(format!(
                "/plugins {} {}",
                if wish { "disable" } else { "enable" },
                p.id
            ))
        }
        "t" if !builtin(p) => Verb::Trust,
        "t" => Verb::Message("built in: compiled into bingo, always full".into()),
        "r" if builtin(p) => Verb::Message(format!("{} is compiled in; nothing to reload", p.id)),
        "r" if matches!(status, PluginState::Off { .. }) => {
            Verb::Message("off; space turns it on".into())
        }
        "r" => Verb::Line("/plugins reload".into()),
        "enter" => Verb::Message(
            if matches!(status, PluginState::Failed { .. }) {
                "r reloads it; the error is on the card"
            } else {
                "space switches it; t changes its trust"
            }
            .into(),
        ),
        _ => Verb::None,
    }
}
pub fn server(s: &ServerRow, key: &str) -> Verb {
    let verb = match key {
        "space" | " " if matches!(s.status, Status::Waiting { .. }) => "approve",
        "space" | " " => {
            if matches!(s.status, Status::Disabled) {
                "enable"
            } else {
                "disable"
            }
        }
        "enter" => match s.status {
            Status::Connected { .. } => "tools",
            Status::NeedsAuth { .. } => "login",
            Status::Failed { .. } => "reconnect",
            Status::Disabled => "enable",
            Status::Waiting { .. } => "approve",
            _ => return Verb::Message("wait for the current connection or sign-in".into()),
        },
        "r" if matches!(s.status, Status::Connected { .. } | Status::Failed { .. }) => "reconnect",
        "r" => {
            return Verb::Message(
                match s.status {
                    Status::NeedsAuth { .. } => "sign in first; ⏎ does it",
                    Status::Disabled => "off; space turns it on",
                    Status::Waiting { .. } => "waits for your yes; ⏎ approves it",
                    _ => "wait for the current connection or sign-in",
                }
                .into(),
            );
        }
        "o" if matches!(s.auth, Some(Auth::SignedIn)) => "logout",
        _ => return Verb::None,
    };
    Verb::Line(format!("/mcp {verb} {}", s.name))
}
