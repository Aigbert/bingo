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
    /// What the row is: a file's short path, a memory's or entry's name.
    pub label: String,
    /// What the row says at its right edge.
    pub note: String,
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
            rows: listing.instructions.into_iter().map(instruction).collect(),
        },
        Group {
            name: "experience",
            rows: entries.into_iter().map(experience).collect(),
        },
    ];
    groups.sort_by_key(|g| g.name != "instructions");
    groups
}
/// A file says where it speaks and how long it is, as the prototype's MEMORY.
fn instruction(row: bingo_services::memory::Loaded) -> Row {
    let reach = match row.scope {
        Scope::User => "every project",
        Scope::Project => "this project",
    };
    Row {
        label: row.path.display().to_string(),
        note: format!("{reach} · {} lines", row.lines),
        tokens: row.tokens,
        target: Target::Instruction(row.path),
    }
}
/// A memory reads as what it says, else its name, and is dated by the day
/// it was written; a broken one says why.
fn memory(row: bingo_services::memory::MemoryRow) -> Row {
    let written = row.written.map(|day| day.strftime("%b %-d").to_string());
    let label = if row.description.is_empty() {
        row.name.clone()
    } else {
        row.description
    };
    Row {
        label,
        note: row.error.or(written).unwrap_or_default(),
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
        label: row.id.clone(),
        note: label,
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
            written: None,
        };
        let groups = rows(
            Listing {
                memories: vec![row],
                instructions: vec![],
            },
            vec![],
        );
        assert!(groups[1].rows[0].note.contains("missing frontmatter"));
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
        let label = &rows(Listing::default(), vec![row])[2].rows[0].note;
        assert_eq!(label, "broken  unreadable  missing frontmatter");
    }
    #[test]
    fn a_memory_reads_as_what_it_says_and_is_dated_by_its_day() {
        let memory = |name: &str| bingo_services::memory::MemoryRow {
            scope: Scope::User,
            name: name.into(),
            kind: bingo_services::memory::Kind::User,
            description: "what it says".into(),
            path: format!("{name}.md").into(),
            tokens: 1,
            error: None,
            written: "2026-09-02".parse().ok(),
        };
        let long = "a".repeat(40);
        let groups = rows(
            Listing {
                memories: vec![memory(&long)],
                instructions: vec![],
            },
            vec![],
        );
        assert_eq!(groups[1].rows[0].label, "what it says");
        assert_eq!(groups[1].rows[0].note, "Sep 2");
        assert!(matches!(
            &groups[1].rows[0].target,
            Target::Memory { name, .. } if *name == long
        ));
    }
}
