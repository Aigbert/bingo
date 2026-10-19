//! Which files under a directory git counts as changed, for the `@` list.
//!
//! Read once per opening of the list, off the UI thread, and bounded on every
//! side: a slow, failed or enormous read lists the files with no state rather
//! than holding a frame.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::time::Duration;

use tokio::io::AsyncReadExt;

/// What git says of one path.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Change {
    Modified,
    New,
}

impl Change {
    /// The word the list sets right of the path, and its ink.
    pub fn word(self) -> (&'static str, &'static str) {
        match self {
            Self::Modified => ("modified", "amber"),
            Self::New => ("new", "good"),
        }
    }
}

/// Paths relative to the directory read, each with its change.
pub type Changes = BTreeMap<String, Change>;

/// How long the read may take before the list gives up on it.
pub const DEADLINE: Duration = Duration::from_secs(2);
/// The most of git's answer read; a status longer than this is cut, and the
/// record the cut lands in is dropped by the parse.
const BYTES: u64 = 1 << 20;
/// The most paths kept.
const ENTRIES: usize = 2_000;

/// The changes under `cwd`, or none when git is missing, the directory is not
/// a repository, or the read is slower than `deadline`.
pub async fn read(cwd: PathBuf, deadline: Duration) -> Changes {
    let Some(prefix) = prefix(&cwd) else {
        return Changes::new();
    };
    match tokio::time::timeout(deadline, status(&cwd)).await {
        Ok(Some(bytes)) => parse(&bytes, &prefix),
        _ => Changes::new(),
    }
}

/// `git status` in porcelain v1 with NUL ends: no quoting, stable across
/// versions. The child is killed when this future is dropped.
async fn status(cwd: &Path) -> Option<Vec<u8>> {
    let mut command = tokio::process::Command::new("git");
    command
        .args([
            "--no-optional-locks",
            "status",
            "--porcelain=v1",
            "-z",
            "--untracked-files=all",
        ])
        .current_dir(cwd)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null());
    let mut child = bingo_process::tree(command).ok()?;
    let mut bytes = Vec::new();
    child
        .take_stdout()?
        .take(BYTES)
        .read_to_end(&mut bytes)
        .await
        .ok()?;
    child.wait().await.ok()?.success().then_some(bytes)
}

/// Where `cwd` sits inside its repository, as git spells a path: empty at the
/// root, `crates/core/` below it. None outside any repository.
pub fn prefix(cwd: &Path) -> Option<String> {
    let root = cwd.ancestors().find(|dir| dir.join(".git").exists())?;
    let below = cwd.strip_prefix(root).ok()?;
    Some(
        below
            .components()
            .map(|part| format!("{}/", part.as_os_str().to_string_lossy()))
            .collect(),
    )
}

/// The records of a `--porcelain=v1 -z` status, as paths under `prefix`
/// relative to it. A rename or copy carries its source as the next record,
/// which is skipped; a record the byte cap cut is dropped.
pub fn parse(bytes: &[u8], prefix: &str) -> Changes {
    let whole = bytes
        .iter()
        .rposition(|byte| *byte == 0)
        .map_or(&bytes[..0], |end| &bytes[..end]);
    let mut records = whole.split(|byte| *byte == 0);
    let mut changes = Changes::new();
    while let Some(record) = records.next() {
        let Some((code, path)) = split(record) else {
            continue;
        };
        if matches!(code[0], b'R' | b'C') {
            records.next();
        }
        if let Some(path) = path.strip_prefix(prefix) {
            changes.insert(path.to_owned(), change(code));
        }
        if changes.len() == ENTRIES {
            break;
        }
    }
    changes
}

/// `XY path` into its code and its path.
fn split(record: &[u8]) -> Option<([u8; 2], &str)> {
    let code = [*record.first()?, *record.get(1)?];
    let path = std::str::from_utf8(record.get(3..)?).ok()?;
    (!path.is_empty()).then_some((code, path))
}

/// Untracked or added is new; anything else git lists is a change.
fn change(code: [u8; 2]) -> Change {
    match code {
        [b'?', b'?'] | [b'A', _] => Change::New,
        _ => Change::Modified,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn untracked_and_added_are_new_and_the_rest_modified() {
        let status = b" M src/lib.rs\0?? tests/new.rs\0A  docs/adr.md\0MM Cargo.toml\0D  gone.rs\0";
        let changes = parse(status, "");
        assert_eq!(changes["src/lib.rs"], Change::Modified);
        assert_eq!(changes["tests/new.rs"], Change::New);
        assert_eq!(changes["docs/adr.md"], Change::New);
        assert_eq!(changes["Cargo.toml"], Change::Modified);
        assert_eq!(changes["gone.rs"], Change::Modified);
    }

    #[test]
    fn a_renames_source_is_not_a_path_of_its_own() {
        let changes = parse(b"R  new.rs\0old.rs\0 M kept.rs\0", "");
        assert_eq!(
            changes.keys().collect::<Vec<_>>(),
            ["kept.rs", "new.rs"],
            "old.rs is the source record"
        );
    }

    #[test]
    fn paths_outside_the_directory_are_dropped_and_the_rest_made_relative() {
        let changes = parse(b" M crates/core/a.rs\0 M crates/tui/b.rs\0", "crates/core/");
        assert_eq!(changes.keys().collect::<Vec<_>>(), ["a.rs"]);
    }

    #[test]
    fn a_record_the_cap_cut_is_dropped() {
        let changes = parse(b" M whole.rs\0 M cut-sh", "");
        assert_eq!(changes.keys().collect::<Vec<_>>(), ["whole.rs"]);
    }

    #[test]
    fn the_count_is_capped() {
        let status = (0..ENTRIES + 10)
            .map(|at| format!("?? f{at:05}.rs\0"))
            .collect::<String>();
        assert_eq!(parse(status.as_bytes(), "").len(), ENTRIES);
    }

    #[test]
    fn the_prefix_is_where_the_directory_sits_below_the_repository() {
        let root = tempfile::tempdir().expect("a directory");
        let below = root.path().join("crates").join("core");
        std::fs::create_dir_all(&below).expect("a nested directory");
        assert_eq!(prefix(&below), None, "no repository yet");
        std::fs::create_dir(root.path().join(".git")).expect("a git directory");
        assert_eq!(prefix(&below).as_deref(), Some("crates/core/"));
        assert_eq!(prefix(root.path()).as_deref(), Some(""));
    }

    #[tokio::test]
    async fn a_directory_outside_a_repository_reads_as_no_changes() {
        let dir = tempfile::tempdir().expect("a directory");
        assert!(read(dir.path().to_path_buf(), DEADLINE).await.is_empty());
    }

    #[tokio::test]
    async fn a_read_past_its_deadline_reads_as_no_changes() {
        let dir = tempfile::tempdir().expect("a directory");
        std::fs::create_dir(dir.path().join(".git")).expect("a git directory");
        std::fs::write(dir.path().join("a.rs"), "").expect("a file");
        assert!(
            read(dir.path().to_path_buf(), Duration::ZERO)
                .await
                .is_empty()
        );
    }
}
