use bingo_mod_memory::{
    draw,
    rows::{self, Target},
};
use bingo_services::{
    experience::EntryRow,
    memory::{Listing, Loaded, MemoryRow, Scope},
};
use ratatui::{Terminal, backend::TestBackend, widgets::Paragraph};

fn listing() -> Listing {
    Listing {
        memories: vec![
            memory(
                Scope::Project,
                "no-sonnet-workers",
                "feedback",
                "Workers are Opus only",
                320,
            ),
            memory(
                Scope::User,
                "role",
                "user",
                "Rust reviewer, prefers concise answers",
                110,
            ),
            memory(
                Scope::Project,
                "wire-fixtures",
                "project",
                "Fixtures before implementation",
                180,
            ),
            memory(
                Scope::User,
                "guide",
                "reference",
                "Read the architecture first",
                130,
            ),
        ],
        instructions: vec![
            Loaded {
                path: "~/.bingo/AGENTS.md".into(),
                tokens: 900,
            },
            Loaded {
                path: "./AGENTS.md".into(),
                tokens: 4200,
            },
        ],
    }
}
fn memory(scope: Scope, name: &str, kind: &str, description: &str, tokens: u64) -> MemoryRow {
    MemoryRow {
        scope,
        name: name.into(),
        kind: kind.into(),
        description: description.into(),
        path: format!("memory/{name}.md").into(),
        tokens,
        error: None,
    }
}
fn entries() -> Vec<EntryRow> {
    ["01J9ABC", "01J9DEF", "01J9GHI"]
        .into_iter()
        .map(|id| EntryRow {
            id: id.into(),
            status: "active".into(),
            summary: "fixture before impl for wire formats".into(),
            outcomes: "3 ok".into(),
            tokens: 80,
        })
        .collect()
}
fn snapshot(width: u16, height: u16, scene: &str) -> String {
    let mut listed = if scene == "empty" {
        Listing::default()
    } else {
        listing()
    };
    if scene == "malformed" {
        listed.memories[0].error = Some("missing frontmatter: description".into());
        listed.memories[0].kind = "malformed".into();
    }
    let groups = rows::rows(listed, if scene == "empty" { vec![] } else { entries() });
    let confirming: Option<&Target> = (scene == "confirming").then(|| &groups[0].rows[0].target);
    let view = draw::tree(&groups, 0, confirming, height);
    let lines = crate::views::render(&view, usize::from(width));
    let mut terminal = Terminal::new(TestBackend::new(width, height)).unwrap();
    terminal
        .draw(|frame| frame.render_widget(Paragraph::new(lines), frame.area()))
        .unwrap();
    let buffer = terminal.backend().buffer();
    buffer
        .content
        .chunks(usize::from(width))
        .map(|row| {
            row.iter()
                .map(|cell| cell.symbol())
                .collect::<String>()
                .trim_end()
                .to_string()
        })
        .collect::<Vec<_>>()
        .join("\n")
}
#[test]
fn memory_panel_sketch_at_both_sizes() {
    for (width, height) in [(108, 30), (80, 24)] {
        for scene in ["full", "empty", "confirming", "malformed"] {
            insta::assert_snapshot!(
                format!("memory_{scene}_{width}x{height}"),
                snapshot(width, height, scene)
            );
        }
    }
}
