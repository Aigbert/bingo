//! A fold belongs to its contributor and is projected only while it is running.
use bingo_sdk::{CommandFold, CommandSpec, ItemBody, Level};

use super::{Host, Mailbox, Registry};

pub(super) struct OwnedCommandFold {
    pub plugin: String,
    pub fold: CommandFold,
}

impl Registry {
    pub(super) fn add_command_fold(&mut self, plugin: &str, fold: CommandFold) {
        self.commands.folds.push(OwnedCommandFold {
            plugin: plugin.into(),
            fold,
        });
    }

    /// Commands can be registered after their folds, including the built-ins.
    pub(super) fn validate_command_folds(&mut self) {
        let specs: Vec<_> = self
            .commands
            .entries
            .iter()
            .map(|entry| entry.spec())
            .collect();
        // Off residents keep their slot so re-enabling cannot create competing folds.
        for owned in std::mem::take(&mut self.commands.folds) {
            if let Some((code, why)) = refusal(&owned.fold, &self.commands.folds, &specs) {
                let text = format!(
                    "{} cannot fold into {}: {why}",
                    owned.plugin, owned.fold.into
                );
                tracing::warn!(plugin = owned.plugin, %text, "command fold refused");
                self.notices.push((code.into(), text));
            } else {
                self.commands.folds.push(owned);
            }
        }
    }

    pub(super) fn folded_into(&self, spec: &CommandSpec) -> Option<&str> {
        let off = self
            .switches
            .running
            .read()
            .unwrap_or_else(|error| error.into_inner());
        // Description links may add aliases after registration was validated.
        // Every active destination stays visible, even if an alias matches a fold.
        if self
            .commands
            .folds
            .iter()
            .any(|owned| !off.contains(&owned.plugin) && owned.fold.into == spec.name)
        {
            return None;
        }
        self.commands
            .folds
            .iter()
            .filter(|owned| !off.contains(&owned.plugin))
            .find(|owned| {
                owned
                    .fold
                    .names
                    .iter()
                    .any(|name| name == &spec.name || spec.aliases.contains(name))
            })
            .map(|owned| owned.fold.into.as_str())
    }
}

fn canonical<'a>(name: &'a str, specs: &'a [CommandSpec]) -> &'a str {
    specs
        .iter()
        .find(|spec| spec.name == name || spec.aliases.iter().any(|alias| alias == name))
        .map_or(name, |spec| spec.name.as_str())
}

fn contains(fold: &CommandFold, name: &str, specs: &[CommandSpec]) -> bool {
    fold.names
        .iter()
        .any(|source| canonical(source, specs) == canonical(name, specs))
}

fn refusal(
    fold: &CommandFold,
    accepted: &[OwnedCommandFold],
    specs: &[CommandSpec],
) -> Option<(&'static str, String)> {
    if fold.into.trim().is_empty() || fold.names.iter().any(|name| name.trim().is_empty()) {
        return Some((
            "COMMAND_FOLD_INVALID",
            "command names must not be empty".into(),
        ));
    }
    if contains(fold, &fold.into, specs) {
        return Some((
            "COMMAND_FOLD_INVALID",
            "a destination cannot fold itself".into(),
        ));
    }
    for held in accepted {
        if let Some(name) = fold
            .names
            .iter()
            .find(|name| contains(&held.fold, name, specs))
        {
            return Some((
                "COMMAND_FOLD_CONFLICT",
                format!("{} already folds {name}", held.plugin),
            ));
        }
        // A fold is one hop to a visible destination, never a hidden chain.
        if contains(&held.fold, &fold.into, specs) || contains(fold, &held.fold.into, specs) {
            return Some((
                "COMMAND_FOLD_INVALID",
                format!(
                    "destinations must stay visible; conflicts with {}",
                    held.plugin
                ),
            ));
        }
    }
    None
}

pub(super) async fn report(host: &Host, mailbox: &Mailbox) {
    for (code, text) in &host.registry.notices {
        if !matches!(
            code.as_str(),
            "COMMAND_FOLD_CONFLICT" | "COMMAND_FOLD_INVALID"
        ) {
            continue;
        }
        let body = ItemBody::Notice {
            level: Level::Warn,
            code: code.clone(),
            text: text.clone(),
        };
        if let Err(error) = mailbox.record(body).await {
            tracing::debug!(%error, code, "a fold refusal reached no transcript");
        }
    }
}
