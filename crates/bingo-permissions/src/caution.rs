//! Warnings describe destructive commands without granting permission.
use bingo_sdk::{Caution, Subject};

pub fn subjects(subjects: &[Subject]) -> Option<Caution> {
    subjects.iter().find_map(|subject| match subject {
        Subject::Command { command } => caution(command),
        _ => None,
    })
}

pub fn caution(command: &str) -> Option<Caution> {
    crate::split::split(command)
        .parts()
        .iter()
        .find_map(|part| simple(part))
        .map(|(act, reason)| Caution {
            reason: reason.into(),
            act: Some(act.into()),
        })
}

fn simple(command: &str) -> Option<(&'static str, &'static str)> {
    let words: Vec<_> = command
        .split_whitespace()
        .map(|word| word.trim_matches(['\'', '"']))
        .collect();
    let mut words = words.as_slice();
    while words
        .first()
        .is_some_and(|word| matches!(*word, "sudo" | "env" | "command"))
    {
        words = &words[1..];
        while let Some(flag) = words.first().filter(|word| word.starts_with('-')) {
            let takes_value = matches!(*flag, "-u" | "-g" | "--user" | "--group");
            words = words.get(if takes_value { 2 } else { 1 }..)?;
        }
    }
    let head = words.first()?.rsplit('/').next()?;
    match head {
        "rm" if words.iter().skip(1).any(|word| {
            *word == "--recursive"
                || (word.starts_with('-')
                    && !word.starts_with("--")
                    && (word.contains('r') || word.contains('R')))
        }) =>
        {
            Some(("delete recursively", "permanently removes files"))
        }
        "git" if words.contains(&"reset") && words.contains(&"--hard") => {
            Some(("hard-reset", "discards uncommitted changes"))
        }
        "git"
            if words.contains(&"push")
                && words
                    .iter()
                    .any(|word| *word == "-f" || word.starts_with("--force")) =>
        {
            Some(("force-push", "rewrites remote history"))
        }
        "git"
            if words.contains(&"clean")
                && words
                    .iter()
                    .any(|word| word.starts_with('-') && word.contains('f')) =>
        {
            Some(("clean", "removes untracked files"))
        }
        "dd" => Some(("overwrite", "irreversibly overwrites data")),
        "mkfs" => Some(("format", "erases the filesystem")),
        "shred" => Some(("shred", "irreversibly overwrites data")),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn every_pattern_names_its_act_and_reason() {
        for (command, act, reason) in [
            (
                "rm -rf x",
                "delete recursively",
                "permanently removes files",
            ),
            (
                "git reset --hard",
                "hard-reset",
                "discards uncommitted changes",
            ),
            (
                "git push --force origin dev",
                "force-push",
                "rewrites remote history",
            ),
            ("git clean -fd", "clean", "removes untracked files"),
            ("dd if=a of=b", "overwrite", "irreversibly overwrites data"),
            ("mkfs /dev/example", "format", "erases the filesystem"),
            ("shred file", "shred", "irreversibly overwrites data"),
        ] {
            assert_eq!(
                caution(command),
                Some(Caution {
                    reason: reason.into(),
                    act: Some(act.into()),
                }),
                "{command}"
            );
        }
    }

    #[test]
    fn destructive_commands_are_warned_including_nested_commands() {
        for command in [
            "rm -rf x",
            "rm -R x",
            "rm --recursive x",
            "git reset --hard",
            "git push --force",
            "git push -f",
            "dd if=a of=b",
            "sudo rm -rf x",
            "cargo test; rm -rf ~",
            "echo $(rm -rf x)",
        ] {
            assert!(caution(command).is_some(), "{command}");
        }
        for command in ["cargo test", "git status", "rm file", "echo 'rm -rf x'"] {
            assert_eq!(caution(command), None, "{command}");
        }
    }
}
