//! Command bindings in declaration order, with the person's settings first.
use crate::CommandSpec;
use std::collections::BTreeMap;

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Bindings {
    pub commands: BTreeMap<String, String>,
    pub notices: Vec<String>,
}

pub fn bindings(
    commands: &[CommandSpec],
    settings: &BTreeMap<String, String>,
    static_keys: &[&str],
) -> Bindings {
    let mut result = Bindings::default();
    for (key, command) in settings {
        claim(&mut result, key, command, static_keys);
    }
    let rebound: Vec<_> = result.commands.values().cloned().collect();
    for command in commands.iter().filter(|command| !command.hidden) {
        let line = format!("/{}", command.name);
        if rebound.contains(&line) {
            continue;
        }
        for key in &command.keys {
            claim(&mut result, key, &line, static_keys);
        }
    }
    result
}

fn claim(result: &mut Bindings, key: &str, command: &str, static_keys: &[&str]) {
    if !command.starts_with('/') || command.trim() == "/" {
        result
            .notices
            .push(format!("{key}: expected a command line, got {command}"));
    } else if static_keys.contains(&key) {
        result
            .notices
            .push(format!("{command}: {key} belongs to the surface"));
    } else if let Some(first) = result.commands.get(key) {
        result
            .notices
            .push(format!("{key}: {command} conflicts with {first}"));
    } else {
        result.commands.insert(key.into(), command.into());
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn command(name: &str, keys: &[&str]) -> CommandSpec {
        let mut spec = CommandSpec::ui(name);
        spec.name = name.into();
        spec.hidden = false;
        spec.keys = keys.iter().map(|key| (*key).into()).collect();
        spec
    }
    #[test]
    fn settings_defaults_conflicts_and_static_keys() {
        let commands = [
            command("hello", &["ctrl+x", "esc"]),
            command("other", &["ctrl+x"]),
        ];
        for (settings, expected, notices) in [
            (
                BTreeMap::new(),
                BTreeMap::from([("ctrl+x".into(), "/hello".into())]),
                2,
            ),
            (
                BTreeMap::from([("alt+h".into(), "/hello".into())]),
                BTreeMap::from([
                    ("alt+h".into(), "/hello".into()),
                    ("ctrl+x".into(), "/other".into()),
                ]),
                0,
            ),
            (
                BTreeMap::from([("esc".into(), "/hello".into())]),
                BTreeMap::from([("ctrl+x".into(), "/hello".into())]),
                3,
            ),
        ] {
            let result = bindings(&commands, &settings, &["esc"]);
            assert_eq!(result.commands, expected);
            assert_eq!(result.notices.len(), notices);
        }
        let result = bindings(&commands, &BTreeMap::new(), &["esc"]);
        assert!(result.notices[1].contains("/hello") && result.notices[1].contains("/other"));
    }
}
