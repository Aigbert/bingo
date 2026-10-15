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
/// Past this a memory name is cut, so one long name does not squeeze every
/// description beside it at 80 columns.
const NAME_WIDTH: usize = 24;
pub fn rows(listing: Listing, entries: Vec<EntryRow>) -> Vec<Group> {
    let name_width = listing
        .memories
        .iter()
        .map(|row| row.name.chars().count().min(NAME_WIDTH))
        .max()
        .unwrap_or(0);
    vec![
        Group {
            name: "memories",
            rows: listing
                .memories
                .into_iter()
                .map(|row| memory(row, name_width))
                .collect(),
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
    ]
}
fn memory(row: bingo_services::memory::MemoryRow, name_width: usize) -> Row {
    Row {
        label: format!(
            "{:<7}  {:<9}  {:name_width$}   {}",
            row.scope.as_str(),
            row.kind.as_str(),
            cut(&row.name, NAME_WIDTH),
            row.error.unwrap_or(row.description)
        ),
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
fn cut(name: &str, width: usize) -> String {
    if name.chars().count() <= width {
        return name.into();
    }
    let kept: String = name.chars().take(width.saturating_sub(1)).collect();
    format!("{kept}…")
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
pub fn path_label(path: &std::path::Path, home: &std::path::Path, cwd: &std::path::Path) -> String {
    if let Ok(relative) = path.strip_prefix(cwd) {
        return format!("./{}", relative.display());
    }
    if let Ok(relative) = path.strip_prefix(home) {
        return format!("~/{}", relative.display());
    }
    path.display().to_string()
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn empty_scopes_and_missing_experience_keep_all_groups() {
        let groups = rows(Listing::default(), vec![]);
        assert_eq!(
            groups.iter().map(|g| g.name).collect::<Vec<_>>(),
            ["memories", "instructions", "experience"]
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
        assert!(groups[0].rows[0].label.contains("missing frontmatter"));
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
    fn a_long_name_is_cut_and_does_not_widen_the_column() {
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
        let cut = format!("{}…", "a".repeat(23));
        assert!(
            groups[0].rows[0]
                .label
                .contains(&format!("{cut}   what it says"))
        );
        assert!(
            groups[0].rows[1]
                .label
                .contains(&format!("short{}   what it says", " ".repeat(19)))
        );
        assert!(matches!(
            &groups[0].rows[0].target,
            Target::Memory { name, .. } if *name == long
        ));
    }
}
