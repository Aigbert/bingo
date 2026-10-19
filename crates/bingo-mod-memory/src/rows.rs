use bingo_services::{
    experience::{EntryRow, Outcome},
    memory::{Listing, Scope},
};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Target {
    Memory {
        scope: Scope,
        name: String,
        path: PathBuf,
    },
    Instruction(PathBuf),
    Experience(String),
}
impl Target {
    pub fn id(&self) -> String {
        match self {
            Self::Memory { path, .. } | Self::Instruction(path) => path.display().to_string(),
            Self::Experience(id) => id.clone(),
        }
    }
    pub fn name(&self) -> String {
        match self {
            Self::Memory { name, .. } | Self::Experience(name) => name.clone(),
            Self::Instruction(path) => path.display().to_string(),
        }
    }
}
#[derive(Clone, Debug)]
pub struct Row {
    pub label: String,
    pub tokens: u64,
    pub target: Target,
}
#[derive(Clone, Debug)]
pub struct Group {
    pub name: &'static str,
    pub rows: Vec<Row>,
}
pub fn rows(listing: Listing, entries: Vec<EntryRow>) -> Vec<Group> {
    let mut groups = vec![
        Group {
            name: "memories",
            rows: listing.memories.into_iter().map(memory).collect(),
        },
        Group {
            name: "instructions",
            rows: listing
                .instructions
                .into_iter()
                .map(|row| Row {
                    label: row.path.display().to_string(),
                    tokens: row.tokens,
                    target: Target::Instruction(row.path),
                })
                .collect(),
        },
        Group {
            name: "experience",
            rows: entries.into_iter().map(experience).collect(),
        },
    ];
    groups.sort_by_key(|g| g.name != "instructions");
    groups
}
fn memory(row: bingo_services::memory::MemoryRow) -> Row {
    Row {
        label: row.error.unwrap_or(row.description),
        tokens: row.tokens,
        target: Target::Memory {
            scope: row.scope,
            name: row.name,
            path: row.path,
        },
    }
}
/// An unreadable entry has no outcomes to count; it says why instead.
fn experience(row: EntryRow) -> Row {
    let label = match &row.error {
        Some(error) => format!("{}  {}  {error}", row.id, row.status.as_str()),
        None => format!(
            "{}  {}  {}  {} ok · {} harmful",
            row.id,
            row.status.as_str(),
            row.summary,
            counted(&row.outcomes, Outcome::Helpful),
            counted(&row.outcomes, Outcome::Harmful),
        ),
    };
    Row {
        label,
        tokens: row.tokens,
        target: Target::Experience(row.id),
    }
}
fn counted(outcomes: &[Outcome], which: Outcome) -> usize {
    outcomes.iter().filter(|outcome| **outcome == which).count()
}
pub fn total(groups: &[Group]) -> u64 {
    groups
        .iter()
        .flat_map(|group| &group.rows)
        .map(|row| row.tokens)
        .fold(0, u64::saturating_add)
}
pub fn about(tokens: u64) -> String {
    if tokens >= 1000 {
        format!("about {:.1}k", tokens as f64 / 1000.0)
    } else {
        format!("about {tokens}")
    }
}
/// A project's file by its name in the project, the person's own from their
/// home, and anything else whole.
pub fn path_label(path: &std::path::Path, home: &std::path::Path, cwd: &std::path::Path) -> String {
    if let Ok(relative) = path.strip_prefix(cwd) {
        return relative.display().to_string();
    }
    if let Ok(relative) = path.strip_prefix(home) {
        return format!("~/{}", relative.display());
    }
    path.display().to_string()
}
#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;
    #[test]
    fn instruction_files_are_named_from_the_project_then_the_home() {
        let (home, cwd) = (Path::new("/home/me"), Path::new("/home/me/work"));
        for (path, label) in [
            ("/home/me/work/AGENTS.md", "AGENTS.md"),
            ("/home/me/.bingo/AGENTS.md", "~/.bingo/AGENTS.md"),
            ("/etc/AGENTS.md", "/etc/AGENTS.md"),
        ] {
            assert_eq!(path_label(Path::new(path), home, cwd), label);
        }
    }
    #[test]
    fn empty_scopes_and_missing_experience_keep_all_groups() {
        let groups = rows(Listing::default(), vec![]);
        assert_eq!(
            groups.iter().map(|g| g.name).collect::<Vec<_>>(),
            ["instructions", "memories", "experience"]
        );
        assert_eq!(total(&groups), 0);
    }
    #[test]
    fn malformed_files_keep_their_error_and_target() {
        let row = bingo_services::memory::MemoryRow {
            scope: Scope::User,
            name: "broken".into(),
            kind: bingo_services::memory::Kind::Malformed,
            description: String::new(),
            path: "broken.md".into(),
            tokens: 8,
            error: Some("missing frontmatter".into()),
        };
        let groups = rows(
            Listing {
                memories: vec![row],
                instructions: vec![],
            },
            vec![],
        );
        assert!(groups[1].rows[0].label.contains("missing frontmatter"));
        assert_eq!(total(&groups), 8);
    }
    #[test]
    fn unreadable_entries_show_why_and_no_counts() {
        let row = EntryRow {
            id: "broken".into(),
            status: bingo_services::experience::Status::Unreadable,
            summary: String::new(),
            outcomes: vec![],
            error: Some("missing frontmatter".into()),
            tokens: 0,
        };
        let label = &rows(Listing::default(), vec![row])[2].rows[0].label;
        assert_eq!(label, "broken  unreadable  missing frontmatter");
    }
    #[test]
    fn a_long_name_keeps_its_identity_and_description() {
        let memory = |name: &str| bingo_services::memory::MemoryRow {
            scope: Scope::User,
            name: name.into(),
            kind: bingo_services::memory::Kind::User,
            description: "what it says".into(),
            path: format!("{name}.md").into(),
            tokens: 1,
            error: None,
        };
        let long = "a".repeat(40);
        let groups = rows(
            Listing {
                memories: vec![memory(&long), memory("short")],
                instructions: vec![],
            },
            vec![],
        );
        assert_eq!(groups[1].rows[0].label, "what it says");
        assert_eq!(groups[1].rows[1].label, "what it says");
        assert!(matches!(
            &groups[1].rows[0].target,
            Target::Memory { name, .. } if *name == long
        ));
    }
}
