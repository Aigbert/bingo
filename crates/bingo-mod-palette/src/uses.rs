//! How often the person ran each command from here, kept in their own data
//! directory — never a project's — and only ever the bare name: an argument
//! may be a key or a secret, and a count needs none of it.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

/// The file under the person's data directory.
const FILE: &str = "plugins/palette/uses.json";
/// How many names the file keeps: enough to rank, never a history.
const KEPT: usize = 32;
/// How many a bare `/` puts in its `recent` group.
pub const SHOWN: usize = 3;

/// One command and how many times it ran.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
struct Use {
    name: String,
    uses: u32,
}

/// The counts, the most recently run first.
pub struct Uses {
    path: PathBuf,
    counts: Vec<Use>,
}

/// Where the counts of the person whose data directory is `data_dir` live.
pub fn path(data_dir: &Path) -> PathBuf {
    data_dir.join(FILE)
}

impl Uses {
    /// The counts on disk; none when the file is missing or unreadable — a
    /// count is a convenience, never a reason not to open.
    pub fn load(path: PathBuf) -> Self {
        let counts = std::fs::read(&path)
            .ok()
            .and_then(|bytes| serde_json::from_slice::<Vec<Use>>(&bytes).ok())
            .unwrap_or_default()
            .into_iter()
            .filter_map(|entry| Some(Use { name: name(&entry.name)?, ..entry }))
            .take(KEPT)
            .collect();
        Self { path, counts }
    }

    /// The names a bare `/` leads with: the most used, a tie going to the
    /// one run last.
    pub fn top(&self) -> Vec<String> {
        let mut ranked: Vec<&Use> = self.counts.iter().collect();
        ranked.sort_by_key(|entry| std::cmp::Reverse(entry.uses));
        ranked
            .into_iter()
            .take(SHOWN)
            .map(|entry| entry.name.clone())
            .collect()
    }

    /// Count one run of `line`, by its name alone, and write the counts back.
    pub fn count(&mut self, line: &str) -> std::io::Result<()> {
        let Some(name) = line.strip_prefix('/').and_then(name) else {
            return Ok(());
        };
        let uses = self
            .counts
            .iter()
            .position(|entry| entry.name == name)
            .map_or(0, |at| self.counts.remove(at).uses);
        self.counts.insert(
            0,
            Use {
                name,
                uses: uses.saturating_add(1),
            },
        );
        self.counts.truncate(KEPT);
        self.write()
    }

    fn write(&self) -> std::io::Result<()> {
        if let Some(parent) = self.path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(&self.path, serde_json::to_vec(&self.counts)?)
    }
}

/// The command a line names, and nothing it was given.
fn name(line: &str) -> Option<String> {
    line.trim_start_matches('/')
        .split_whitespace()
        .next()
        .map(str::to_owned)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn counted(lines: &[&str]) -> (tempfile::TempDir, Uses) {
        let dir = tempfile::tempdir().unwrap();
        let mut uses = Uses::load(path(dir.path()));
        for line in lines {
            uses.count(line).unwrap();
        }
        (dir, uses)
    }

    #[test]
    fn the_counts_live_in_the_person_s_data_directory() {
        let data = Path::new("/home/someone/.local/share/bingo");
        let at = path(data);
        assert!(at.starts_with(data), "{at:?}");
        assert!(
            !at.components().any(|part| part.as_os_str() == ".bingo"),
            "never a project's directory: {at:?}"
        );
    }

    #[test]
    fn the_most_used_lead_and_a_tie_goes_to_the_last_run() {
        let (dir, _) = counted(&["/think", "/model", "/think", "/rename x", "/model", "/status"]);
        let reopened = Uses::load(path(dir.path()));
        assert_eq!(reopened.top(), ["model", "think", "status"]);
    }

    #[test]
    fn arguments_never_reach_the_file() {
        let (dir, uses) = counted(&[
            "/provider add private --key provider-secret",
            "/login pasted-login-secret",
        ]);
        let disk = std::fs::read_to_string(path(dir.path())).unwrap();
        assert!(!disk.contains("secret"), "{disk}");
        assert_eq!(uses.top(), ["login", "provider"]);
    }

    #[test]
    fn a_name_on_disk_is_cut_to_its_command_before_it_is_used() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(path(dir.path()).parent().unwrap()).unwrap();
        std::fs::write(
            path(dir.path()),
            r#"[{"name":"/provider add --key old-secret","uses":2}]"#,
        )
        .unwrap();
        let mut uses = Uses::load(path(dir.path()));
        uses.count("/status").unwrap();
        let disk = std::fs::read_to_string(path(dir.path())).unwrap();
        assert!(!disk.contains("secret"), "{disk}");
        assert_eq!(uses.top(), ["provider", "status"]);
    }

    #[test]
    fn a_long_life_keeps_a_bounded_file() {
        let lines: Vec<String> = (0..100).map(|n| format!("/c{n}")).collect();
        let lines: Vec<&str> = lines.iter().map(String::as_str).collect();
        let (dir, _) = counted(&lines);
        assert_eq!(Uses::load(path(dir.path())).counts.len(), KEPT);
    }
}
