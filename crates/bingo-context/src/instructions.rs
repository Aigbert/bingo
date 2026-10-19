//! The instruction files a project leaves for whoever works in it.

use std::path::{Path, PathBuf};

use async_trait::async_trait;
use bingo_sdk::{
    ContextContributor, ContextError, ContextPiece, ContextQuery, Placement, SystemBlock,
};

use crate::{baseline, files, root};

/// The file a directory speaks through, in order of preference: a project that
/// has written for this agent is not also asked what it told another one.
pub(crate) const NAMES: [&str; 2] = ["AGENTS.md", "CLAUDE.md"];

/// Before everything else in the system prompt: instructions are the frame the
/// rest of the prompt is read in.
const ORDER: i32 = -10;

pub(crate) const ID: &str = "context:instructions";

/// Contributes the user's instructions and every project file from the root
/// down to the working directory.
#[derive(Debug, Clone)]
pub struct InstructionsContributor {
    config_dir: PathBuf,
}

impl InstructionsContributor {
    pub fn new(config_dir: PathBuf) -> Self {
        Self { config_dir }
    }

    pub async fn loaded(&self, cwd: &Path) -> Vec<bingo_services::memory::Loaded> {
        let mut loaded = Vec::new();
        for path in self.paths(cwd).await {
            if let Some(text) = read(&path).await {
                loaded.push(self.loaded_one(path, &text));
            }
        }
        loaded
    }

    fn loaded_one(&self, path: PathBuf, text: &str) -> bingo_services::memory::Loaded {
        use bingo_services::memory::Scope;
        let scope = if path.starts_with(&self.config_dir) {
            Scope::User
        } else {
            Scope::Project
        };
        bingo_services::memory::Loaded {
            tokens: crate::estimate::text(&block(&path, text).text),
            lines: text.lines().count() as u64,
            scope,
            path,
        }
    }

    /// The user's file first, then one file per directory from the root down:
    /// the nearer a file is to the work, the later it speaks.
    async fn paths(&self, cwd: &Path) -> Vec<PathBuf> {
        let root = root::of(cwd).await;
        let mut paths = vec![self.config_dir.join(NAMES[0])];
        for dir in root::chain(&root, cwd) {
            paths.extend(present(&dir).await);
        }
        paths
    }
}

/// The one file this directory speaks through, if it has one.
async fn present(dir: &Path) -> Option<PathBuf> {
    for name in NAMES {
        let path = dir.join(name);
        if tokio::fs::try_exists(&path).await.unwrap_or(false) {
            return Some(path);
        }
    }
    None
}

/// A file that cannot be read is a file that is not there: an unreadable
/// AGENTS.md must not cost the turn.
async fn read(path: &Path) -> Option<String> {
    let text = tokio::fs::read_to_string(path).await.ok()?;
    (!text.trim().is_empty()).then_some(text)
}

fn block(path: &Path, text: &str) -> SystemBlock {
    let heading = format!("# Instructions from {}", path.display());
    files::block(&heading, text, true, files::MAX_LINES)
}

#[async_trait]
impl ContextContributor for InstructionsContributor {
    fn id(&self) -> &str {
        ID
    }

    fn placement(&self) -> Placement {
        Placement::System { order: ORDER }
    }

    async fn contribute(&self, query: ContextQuery<'_>) -> Result<Vec<ContextPiece>, ContextError> {
        baseline::contribute(self.id(), query, async {
            let mut blocks = Vec::new();
            for path in self.paths(query.cwd).await {
                if let Some(text) = read(&path).await {
                    blocks.push(block(&path, &text));
                }
            }
            blocks
        })
        .await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::baseline::testing::Journal;
    use crate::git::{self, Repo};

    fn headings(pieces: &[ContextPiece]) -> Vec<String> {
        pieces
            .iter()
            .filter_map(|p| match p {
                ContextPiece::System(block) => block.text.lines().next().map(str::to_string),
                _ => None,
            })
            .collect()
    }

    async fn contribute(config_dir: &Path, cwd: &Path) -> Vec<ContextPiece> {
        Journal::at(cwd)
            .contribute(&InstructionsContributor::new(config_dir.to_path_buf()), cwd)
            .await
    }

    #[test]
    fn it_is_the_first_thing_in_the_system_prompt() {
        let contributor = InstructionsContributor::new(PathBuf::from("/config"));
        assert_eq!(contributor.id(), "context:instructions");
        assert_eq!(contributor.placement(), Placement::System { order: -10 });
    }

    #[tokio::test]
    async fn every_directory_from_the_root_down_speaks_in_order() {
        let Some(repo) = Repo::init() else {
            return git::absent();
        };
        let config = tempfile::tempdir().expect("a config dir");
        std::fs::write(config.path().join("AGENTS.md"), "be brief\n").expect("the user file");
        repo.write("AGENTS.md", "the project is a kernel\n");
        repo.write("crates/inner/AGENTS.md", "this crate is the inner one\n");
        let cwd = repo.dir("crates/inner");

        let pieces = contribute(config.path(), &cwd).await;
        assert_eq!(
            headings(&pieces),
            [
                format!(
                    "# Instructions from {}",
                    config.path().join("AGENTS.md").display()
                ),
                format!(
                    "# Instructions from {}",
                    repo.root().join("AGENTS.md").display()
                ),
                format!("# Instructions from {}", cwd.join("AGENTS.md").display()),
            ]
        );
    }

    #[tokio::test]
    async fn a_directory_with_both_files_speaks_through_agents_md() {
        let Some(repo) = Repo::init() else {
            return git::absent();
        };
        let config = tempfile::tempdir().expect("a config dir");
        repo.write("AGENTS.md", "the agents file\n");
        repo.write("CLAUDE.md", "the claude file\n");

        let pieces = contribute(config.path(), &repo.root()).await;
        assert_eq!(pieces.len(), 1);
        assert!(text(&pieces[0]).contains("the agents file"));
    }

    #[tokio::test]
    async fn a_directory_with_only_claude_md_still_speaks() {
        let Some(repo) = Repo::init() else {
            return git::absent();
        };
        let config = tempfile::tempdir().expect("a config dir");
        repo.write("CLAUDE.md", "the claude file\n");

        let pieces = contribute(config.path(), &repo.root()).await;
        assert_eq!(pieces.len(), 1);
        assert!(text(&pieces[0]).contains("the claude file"));
    }

    #[tokio::test]
    async fn a_file_that_is_not_there_contributes_nothing() {
        let dir = tempfile::tempdir().expect("a temp dir");
        let config = tempfile::tempdir().expect("a config dir");
        let pieces = contribute(config.path(), dir.path()).await;
        assert!(pieces.is_empty());
    }

    #[tokio::test]
    async fn a_long_file_contributes_its_newest_lines_and_says_so() {
        let dir = tempfile::tempdir().expect("a temp dir");
        let config = tempfile::tempdir().expect("a config dir");
        let long: String = (1..=400).map(|i| format!("rule {i}\n")).collect();
        std::fs::write(dir.path().join("AGENTS.md"), long).expect("the file");

        let pieces = contribute(config.path(), dir.path()).await;
        let text = text(&pieces[0]);
        assert!(text.contains("[… 100 earlier lines not shown]"));
        assert!(text.contains("rule 101") && text.contains("rule 400"));
        assert!(!text.contains("rule 100\n"));
    }

    fn text(piece: &ContextPiece) -> String {
        match piece {
            ContextPiece::System(block) => block.text.clone(),
            ContextPiece::User { .. } => String::new(),
        }
    }
}

#[cfg(test)]
mod measured_tests {
    use super::*;
    #[tokio::test]
    async fn loaded_files_follow_the_contributors_order_and_measure_its_blocks() {
        let home = tempfile::tempdir().unwrap();
        let config = home.path().join("config");
        let cwd = home.path().join("work");
        std::fs::create_dir(&config).unwrap();
        std::fs::create_dir(&cwd).unwrap();
        let paths = [config.join("AGENTS.md"), cwd.join("CLAUDE.md")];
        for path in &paths {
            std::fs::write(path, "use fixtures\n").unwrap();
        }
        let loaded = InstructionsContributor::new(config).loaded(&cwd).await;
        assert_eq!(
            loaded
                .iter()
                .map(|file| file.path.canonicalize().unwrap())
                .collect::<Vec<_>>(),
            paths
                .iter()
                .map(|path| path.canonicalize().unwrap())
                .collect::<Vec<_>>()
        );
        let scopes: Vec<_> = loaded.iter().map(|file| file.scope.clone()).collect();
        use bingo_services::memory::Scope;
        assert_eq!(scopes, [Scope::User, Scope::Project]);
        assert!(loaded.iter().all(|file| file.lines == 1));
        for file in loaded {
            assert_eq!(
                file.tokens,
                crate::estimate::text(&block(&file.path, &read(&file.path).await.unwrap()).text)
            );
        }
    }
}
