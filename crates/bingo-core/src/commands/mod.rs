//! The kernel's own commands (ADR-0008 §4, ADR-0012 §5): what changes the
//! next turn's model or thinking, what a session is called, what makes room,
//! and a provider's credential. They are registered like any plugin's and listed in the same
//! catalogue.

mod compact;
mod login;
mod model;
mod models;
mod pin;
mod plugins;
mod provider;
mod rename;
mod status;
mod think;

use std::sync::{Arc, Weak};

use bingo_sdk::*;

use crate::host::Host;

pub(crate) fn builtins(host: Weak<Host>) -> Vec<Arc<dyn Command>> {
    vec![
        Arc::new(model::ModelCommand { host: host.clone() }),
        Arc::new(models::ModelsCommand { host: host.clone() }),
        Arc::new(think::ThinkCommand { host: host.clone() }),
        Arc::new(rename::RenameCommand { host: host.clone() }),
        Arc::new(compact::CompactCommand { host: host.clone() }),
        Arc::new(login::LoginCommand { host: host.clone() }),
        Arc::new(login::LogoutCommand { host: host.clone() }),
        Arc::new(plugins::PluginsCommand { host: host.clone() }),
        Arc::new(provider::ProviderCommand { host: host.clone() }),
        Arc::new(status::StatusCommand { host }),
    ]
}

/// Write what a command chose into the user settings layer, so the next start
/// opens on it (ADR-0003 §5). The session has already moved; a file that will
/// not take the note says so out loud rather than undoing what the person
/// asked for. `None` is written.
fn remember(host: &Host, keys: &[(&str, serde_json::Value)]) -> Option<String> {
    let path = crate::settings::user_path(host.env());
    crate::settings::remember(&path, keys)
        .err()
        .map(|e| format!("this session has it, but the next will not: {e}"))
}

fn host(weak: &Weak<Host>) -> Result<Arc<Host>, KernelError> {
    weak.upgrade()
        .ok_or_else(|| KernelError::new(ErrorCode::SessionClosed, "the host is shut down"))
}

fn spec(name: &str, hint: &str, args: ArgSpec, instant: bool) -> CommandSpec {
    CommandSpec {
        keys: vec![],
        hidden: false,
        name: name.into(),
        aliases: Vec::new(),
        hint: hint.into(),
        args,
        instant,
        family: "session".into(),
    }
}

/// A single value and an optional trailing persistence flag.
fn choice_args(args: &str) -> Result<(&str, bool), KernelError> {
    let words: Vec<_> = args.split_whitespace().collect();
    match words.as_slice() {
        [value] if !value.starts_with("--") => Ok((value, false)),
        [value, "--keep"] if !value.starts_with("--") => Ok((value, true)),
        _ => Err(KernelError::new(
            ErrorCode::InvalidInput,
            "expected a value followed by optional --keep",
        )),
    }
}

fn keep_args(args: ArgSpec) -> ArgSpec {
    ArgSpec::Options {
        args: Box::new(args),
        flags: vec!["--keep".into()],
    }
}

fn kept(host: &Host, keep: bool, keys: &[(&str, serde_json::Value)]) -> String {
    if !keep {
        return "this session".into();
    }
    remember(host, keys).unwrap_or_else(|| "kept for new sessions".into())
}
