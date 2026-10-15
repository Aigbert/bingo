use bingo_mod_memory::{
    draw,
    rows::{self, Target},
};
use bingo_services::{
    experience::{EntryRow, Outcome, Status},
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
        kind: serde_json::from_value(serde_json::json!(kind)).unwrap(),
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
            status: Status::Active,
            summary: "fixture before impl for wire formats".into(),
            outcomes: vec![Outcome::Helpful; 3],
            error: None,
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
        listed.memories[0].kind = bingo_services::memory::Kind::Malformed;
        listed.memories[1].name = "a-memory-name-far-past-the-column-cap".into();
    }
    let mut entries = if scene == "empty" { vec![] } else { entries() };
    if scene == "malformed" {
        entries[2].status = Status::Unreadable;
        entries[2].error = Some("missing frontmatter".into());
        entries[2].outcomes.clear();
        entries[2].tokens = 0;
    }
    let groups = rows::rows(listed, entries);
    let confirming: Option<&Target> = (scene == "confirming").then(|| &groups[0].rows[0].target);
    let view = draw::tree(&groups, 0, confirming, height, width);
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

#[test]
fn memory_and_experience_frontmatter_stays_verbatim_in_the_pager() {
    use crate::{pager::Pager, test_support::*, ui::Open};
    for (width, height) in [(80, 24), (108, 30)] {
        for (text, fields) in [
            (
                "---\nname: rule\ntype: feedback\ndescription: Use fixtures\n---\n\n**Body stays markdown.**",
                vec!["name: rule", "type: feedback", "description: Use fixtures"],
            ),
            (
                "---\nstatus: active\ntrigger:\n  - wire\nsteps:\n  - write fixture\ncreated: 2026-10-15\n---\n\n**Body stays markdown.**",
                vec![
                    "status: active",
                    "trigger:",
                    "  - wire",
                    "steps:",
                    "  - write fixture",
                    "created: 2026-10-15",
                ],
            ),
        ] {
            let tree = solo(&state());
            let (mut ui, now) = scene();
            let now = still(now);
            ui.focus.show(
                Open::Pager(Pager::text(
                    "file.md".into(),
                    bingo_mod_memory::fence::document(text),
                    None,
                )),
                now.instant,
            );
            let screen = draw_tree(width, height, &tree, &mut ui, now);
            for field in fields {
                assert!(
                    screen
                        .lines()
                        .any(|line| line.trim_end_matches('"').trim_end().ends_with(field)),
                    "{field}: {screen}"
                );
            }
            assert!(screen.contains("Body stays markdown."));
            assert!(!screen.contains("**Body"));
        }
    }
}
#[test]
fn a_pager_renders_other_text_as_given_without_a_fence() {
    use crate::{pager::Pager, test_support::*, ui::Open};
    let tree = solo(&state());
    let (mut ui, now) = scene();
    let now = still(now);
    ui.focus.show(
        Open::Pager(Pager::text(
            "notes".into(),
            "---\ntitle: plain notes\n---\n\n**Body**".into(),
            None,
        )),
        now.instant,
    );
    let screen = draw_tree(80, 24, &tree, &mut ui, now);
    assert!(
        !screen
            .lines()
            .any(|line| line.trim_matches('"').trim() == "---"),
        "a dash rule outside the memory panel stays Markdown: {screen}"
    );
    assert!(screen.contains("title: plain notes"), "{screen}");
}
#[test]
fn a_pager_does_not_return_focus_to_a_removed_pane() {
    use crate::{pager::Pager, test_support::*, ui::Open};
    let tree = solo(&state());
    let (mut ui, now) = scene();
    ui.focus.show(
        Open::Pager(Pager::text(
            "file".into(),
            "text".into(),
            Some(crate::pager::Return::Pane("removed".into())),
        )),
        now.instant,
    );
    crate::pager::keys(&mut ui, &tree, key(crossterm::event::KeyCode::Esc), now);
    assert!(!matches!(ui.focus, crate::focus::Focus::Pane(_)));
    assert!(!ui.focus.showing());
}
