//! The files the `@` list ranks: one walk of the session's directory, kept
//! until the directory changes, and one git read for each opening of the list.

use std::path::PathBuf;

use super::Entry;
use crate::git_changes::Changes;

#[derive(Debug, Default)]
pub struct Files {
    cwd: String,
    paths: Vec<String>,
    /// Whether the line ended in a mention when it was last looked at.
    open: bool,
    /// Counts the openings, so a read that answers a closed one is dropped.
    opening: u64,
    git: Git,
}

/// Where this opening's git read stands.
#[derive(Debug, Default)]
enum Git {
    #[default]
    Unwanted,
    Wanted,
    Asked,
    Read(Changes),
}

impl Files {
    /// Look at the line before a frame or a key: walk the directory the first
    /// time a mention asks in it, and want a git read when the list opens.
    pub fn prepare(&mut self, line: &str, cwd: &str) {
        let open = crate::complete::mention(line).is_some();
        if open && self.cwd != cwd {
            self.cwd = cwd.to_owned();
            self.paths = crate::complete::walk(std::path::Path::new(cwd));
        }
        if open && !self.open {
            self.opening += 1;
            self.git = Git::Wanted;
        }
        if !open {
            self.git = Git::Unwanted;
        }
        self.open = open;
    }

    /// The read this opening wants, once: its number and the directory.
    pub fn ask(&mut self) -> Option<(u64, PathBuf)> {
        if !matches!(self.git, Git::Wanted) {
            return None;
        }
        self.git = Git::Asked;
        Some((self.opening, PathBuf::from(&self.cwd)))
    }

    /// A read came back; it counts only for the opening that asked.
    pub fn answered(&mut self, opening: u64, changes: Changes) {
        if opening == self.opening && matches!(self.git, Git::Asked) {
            self.git = Git::Read(changes);
        }
    }

    /// Every walked path, with git's word on it where the read has one.
    pub fn entries(&self) -> impl Iterator<Item = Entry> + '_ {
        let changes = match &self.git {
            Git::Read(changes) => Some(changes),
            _ => None,
        };
        self.paths.iter().map(move |path| Entry::File {
            path: path.clone(),
            change: changes.and_then(|changes| changes.get(path).copied()),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::git_changes::Change;

    fn changes(path: &str) -> Changes {
        Changes::from([(path.to_owned(), Change::Modified)])
    }

    #[test]
    fn one_read_is_asked_for_each_opening() {
        let mut files = Files::default();
        files.prepare("look at @", "/nowhere");
        assert_eq!(files.ask().map(|(at, _)| at), Some(1));
        assert_eq!(files.ask(), None, "asked once");
        files.prepare("look at @qu", "/nowhere");
        assert_eq!(files.ask(), None, "typing on is the same opening");
        files.prepare("look at @queue.rs ", "/nowhere");
        files.prepare("look at @queue.rs @", "/nowhere");
        assert_eq!(files.ask().map(|(at, _)| at), Some(2));
    }

    #[test]
    fn a_read_for_a_closed_opening_is_dropped() {
        let mut files = Files {
            paths: vec!["a.rs".into()],
            ..Files::default()
        };
        files.cwd = "/nowhere".into();
        files.prepare("@", "/nowhere");
        let (first, _) = files.ask().expect("the first opening asks");
        files.prepare("", "/nowhere");
        files.prepare("@", "/nowhere");
        files.answered(first, changes("a.rs"));
        assert!(
            files
                .entries()
                .all(|entry| matches!(entry, Entry::File { change: None, .. }))
        );
        let (second, _) = files.ask().expect("the second opening asks");
        files.answered(second, changes("a.rs"));
        assert!(files.entries().any(|entry| matches!(
            entry,
            Entry::File {
                change: Some(Change::Modified),
                ..
            }
        )));
    }
}
