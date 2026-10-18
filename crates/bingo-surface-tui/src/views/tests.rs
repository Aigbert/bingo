//! Every node of the vocabulary, drawn once and read as a screen (ADR-0013
//! §1, design §9). A node enters the TUI with its screen here or it does not
//! enter: [`named`] is an exhaustive match, so a fifteenth node would not
//! compile until somebody had looked at it.

use bingo_sdk::{Action, ActionItem, Tone, TreeNode, View};
use ratatui::Terminal;
use ratatui::backend::TestBackend;
use ratatui::widgets::Paragraph;

use super::*;

/// One node on a screen of its own, as many rows as it draws.
fn screen(view: &View, width: u16) -> String {
    let lines = render(view, usize::from(width));
    let height = u16::try_from(lines.len().max(1)).unwrap_or(u16::MAX);
    let mut terminal = Terminal::new(TestBackend::new(width, height)).expect("a test terminal");
    terminal
        .draw(|frame| frame.render_widget(Paragraph::new(lines), frame.area()))
        .expect("a drawn frame");
    terminal.backend().to_string()
}

/// The width every rule in design §5 is written for.
fn at_80(view: &View) -> String {
    screen(view, 80)
}

fn action(name: &str) -> Action {
    Action {
        name: name.into(),
        args: serde_json::Value::Null,
    }
}

fn table() -> View {
    View::Table {
        headers: vec!["id".into(), "task".into(), "tokens".into()],
        rows: vec![
            vec!["1".into(), "write the plan".into(), "1200".into()],
            vec!["2".into(), "ship it".into()],
        ],
    }
}

fn actions() -> View {
    View::Actions {
        items: vec![
            ActionItem {
                meaning: None,
                label: "Approve".into(),
                action: action("board.tick"),
                key: None,
            },
            ActionItem {
                meaning: None,
                label: "Next".into(),
                action: action("board.next"),
                key: None,
            },
            ActionItem {
                meaning: None,
                label: "Skip".into(),
                action: action("board.skip"),
                key: Some('s'),
            },
        ],
    }
}

fn tree() -> View {
    View::Tree {
        nodes: vec![TreeNode {
            label: "project".into(),
            badge: None,
            tone: Tone::Neutral,
            children: vec![
                TreeNode {
                    label: "reviewer".into(),
                    badge: Some("waiting".into()),
                    tone: Tone::Attention,
                    children: Vec::new(),
                },
                TreeNode {
                    label: "scout".into(),
                    badge: Some("done".into()),
                    tone: Tone::Good,
                    children: vec![TreeNode {
                        label: "Read(Cargo.toml)".into(),
                        badge: None,
                        tone: Tone::Neutral,
                        children: Vec::new(),
                    }],
                },
            ],
        }],
    }
}

fn columns() -> View {
    View::columns(vec![
        View::KeyValue {
            rows: vec![
                ("model".into(), "gpt-5.4".into()),
                ("rounds".into(), "12".into()),
            ],
        },
        View::List {
            items: vec!["one".into(), "two".into(), "three".into()],
        },
    ])
}

/// Every node, once. The match is exhaustive on purpose.
fn named(view: &View) -> &'static str {
    match view {
        View::Text { .. } => "text",
        View::Markdown { .. } => "markdown",
        View::Code { .. } => "code",
        View::Diff { .. } => "diff",
        View::List { .. } => "list",
        View::Table { .. } => "table",
        View::KeyValue { .. } => "keyvalue",
        View::Progress { .. } => "progress",
        View::Badge { .. } => "badge",
        View::Tree { .. } => "tree",
        View::Box { layout, .. }
            if layout.flex_direction == Some(bingo_sdk::view::Direction::Row) =>
        {
            "columns"
        }
        View::Box { .. } => "stack",
        View::Panel { .. } => "panel",
        View::Actions { .. } => "actions",
        View::Custom { .. } => "custom",
        View::Button { .. } => "button",
        View::Input { .. } => "input",
        View::Select { .. } => "select",
        View::Link { .. } => "link",
        View::Image { .. } => "image",
        View::Raster { .. } => "raster",
        View::Svg { .. } => "svg",
        View::Engine { .. } => "engine",
    }
}

/// A word this surface has not learned (ADR-0038): the fold is the whole of
/// what it draws, and the data it cannot read is carried past it untouched.
fn custom() -> View {
    View::Custom {
        kind: "demo.sparkline".into(),
        data: serde_json::json!({"points": [3, 5, 8, 13]}),
        fold: "3 5 8 13".into(),
    }
}

fn every_node() -> Vec<View> {
    vec![
        View::text("what a plugin wrote"),
        View::Markdown {
            text: "# Title\n\nbody with **weight** and a [link](https://x.dev)".into(),
            dim_color: false,
        },
        View::Code {
            language: Some("rust".into()),
            source: (1..=10)
                .map(|i| format!("let line_{i} = {i};\n"))
                .collect::<String>(),
        },
        View::Diff {
            unified: "@@ -1,2 +1,2 @@\n-let a = 1;\n+let a = 2;\n ok\n".into(),
        },
        View::List {
            items: vec!["write the plan".into(), "ship it".into()],
        },
        table(),
        View::KeyValue {
            rows: vec![
                ("model".into(), "gpt-5.4".into()),
                ("rounds".into(), "12".into()),
                ("branch".into(), String::new()),
            ],
        },
        View::Progress {
            value: 24,
            total: Some(30),
            label: Some("cargo test".into()),
        },
        View::Badge {
            text: "needs you".into(),
            tone: Tone::Attention,
        },
        tree(),
        View::column(vec![View::text("above"), View::text(""), table()]),
        columns(),
        View::Panel {
            title: "Board".into(),
            child: Box::new(table()),
        },
        actions(),
        custom(),
    ]
}

#[test]
fn every_node_of_the_vocabulary_has_a_screen_of_its_own() {
    let mut names: Vec<&str> = every_node().iter().map(named).collect();
    let all = names.len();
    names.sort_unstable();
    names.dedup();
    assert_eq!(names.len(), all, "one screen per node, and no node twice");
    for view in every_node() {
        insta::assert_snapshot!(named(&view), at_80(&view));
    }
}

/// Two columns of thirty cells read worse than one of sixty, so below the
/// narrow width they stack instead (design §7's readable widths).
#[test]
fn columns_stack_below_sixty_cells() {
    insta::assert_snapshot!("columns_at_50", screen(&columns(), 50));
}

#[test]
fn an_unbounded_progress_shows_its_count_and_not_a_fraction() {
    let view = View::Progress {
        value: 7,
        total: None,
        label: Some("indexing".into()),
    };
    assert_eq!(
        render(&view, 80)[0].to_string().trim_end(),
        "███░░░░░░░ 7 · indexing"
    );
}

/// A bar that has no fraction to show says the work is alive by walking its
/// three lit cells along the track (design §5). The frame's own beat is what
/// moves it, so a still surface — and a record, which is what [`render`]
/// draws — leaves it at the head.
#[test]
fn an_unbounded_bar_walks_its_sheen_along_the_track() {
    let bar = |beat| {
        progress::lines(7, None, None, 80, beat)[0]
            .to_string()
            .trim_end()
            .to_string()
    };
    assert_eq!(bar(0.0), "███░░░░░░░ 7", "parked at the head");
    assert_eq!(bar(0.5), "░░░░░███░░ 7", "half a turn along");
    assert_eq!(bar(0.9), "██░░░░░░░█ 7", "and it wraps, never falls off");
    assert_eq!(bar(1.0), bar(0.0), "one whole turn is back where it began");
}

#[test]
fn a_missing_cell_is_a_dash_and_numbers_hug_their_right_edge() {
    let drawn: Vec<String> = render(&table(), 80)
        .iter()
        .map(|line| line.to_string().trim_end().to_string())
        .collect();
    assert_eq!(drawn[0], "id  task            tokens");
    assert_eq!(drawn[2], " 1  write the plan    1200");
    assert_eq!(
        drawn[3], " 2  ship it              –",
        "a row that is short"
    );
}

#[test]
fn a_row_wider_than_the_frame_is_cut_and_says_so() {
    let view = View::Table {
        headers: vec!["path".into()],
        rows: vec![vec!["a".repeat(40)]],
    };
    let drawn = render(&view, 20);
    assert_eq!(drawn[2].to_string(), format!("{}…", "a".repeat(19)));
}

#[test]
fn a_key_is_the_plugins_hint_else_where_the_button_sits() {
    let View::Actions { items } = actions() else {
        panic!("actions");
    };
    let offered: Vec<&ActionItem> = items.iter().collect();
    assert_eq!(actions::key_of(&items[0], 0), '1');
    assert_eq!(actions::key_of(&items[2], 2), 's', "the plugin named one");
    assert_eq!(
        actions::fired(&offered, '2').map(|item| item.label.as_str()),
        Some("Next")
    );
    assert_eq!(actions::fired(&offered, '9'), None);
}

#[test]
fn a_fired_button_wears_the_mark_until_the_answer_comes_back() {
    let marks = Marks {
        pending: Some(action("board.tick")),
        ..Marks::default()
    };
    let drawn = marked(&actions(), 80, &marks).lines[0].to_string();
    assert!(drawn.contains("[ 1 Approve… ]"), "{drawn}");
    assert!(drawn.contains("[ 2 Next ]"), "{drawn}");
}

#[test]
fn buttons_that_do_not_fit_the_width_go_on_the_next_row() {
    let drawn = render(&actions(), 24);
    assert_eq!(drawn.len(), 2, "{drawn:?}");
}

#[test]
fn canonical_elements_have_terminal_screens() {
    use serde_json::json;
    let cases = [
        (
            json!({"kind":"Box","padding":1,"borderStyle":"single","children":[{"kind":"Text","children":["inside"]}]}),
            "inside",
        ),
        (json!({"kind":"Text","children":["styled"]}), "styled"),
        (
            json!({"kind":"Button","hotkey":"g","label":"Go","onPress":{"name":"counter:ui","args":{}}}),
            "g Go",
        ),
        (
            json!({"kind":"Input","label":"Name","value":"Ada"}),
            "Name: Ada",
        ),
        (
            json!({"kind":"Select","label":"Mode","value":"fast","options":[{"value":"fast","label":"Fast"}]}),
            "Fast",
        ),
        (
            json!({"kind":"Link","href":"https://example.test","label":"Example"}),
            "Example",
        ),
        (
            json!({"kind":"Image","source":"blob:missing","alt":"A cat"}),
            "A cat",
        ),
        (
            json!({"kind":"Raster","columns":2,"rows":1,"cells":["ab"]}),
            "ab",
        ),
        (
            json!({"kind":"Svg","source":"<svg/>","alt":"A circle"}),
            "A circle",
        ),
        (json!({"kind":"Engine","component":"Pane","props":{}}), ""),
        (
            json!({"kind":"Code","source":"let x = 1;","language":"rust"}),
            "let x = 1;",
        ),
        (
            json!({"kind":"Markdown","text":"**weight**","dimColor":true}),
            "weight",
        ),
    ];
    for (wire, words) in cases {
        let view: View = serde_json::from_value(wire).unwrap();
        assert!(
            screen(&view, 80).contains(words),
            "{}: {words}",
            named(&view)
        );
    }
}

#[test]
fn text_children_keep_their_own_weight_in_the_backend() {
    let view: View = serde_json::from_value(serde_json::json!({"kind":"Text","children":["a",{"kind":"Text","bold":true,"children":["b"]},"c"]})).unwrap();
    let mut terminal = Terminal::new(TestBackend::new(3, 1)).unwrap();
    terminal
        .draw(|frame| frame.render_widget(Paragraph::new(render(&view, 3)), frame.area()))
        .unwrap();
    let buffer = terminal.backend().buffer();
    assert_eq!(
        buffer[(1, 0)].style(),
        theme::as_drawn(buffer[(0, 0)].style().patch(theme::bold()))
    );
    assert_eq!(buffer[(0, 0)].style(), buffer[(2, 0)].style());
}

#[test]
fn box_padding_and_hidden_children_affect_drawn_geometry() {
    let view: View = serde_json::from_value(serde_json::json!({"kind":"Box","width":8,"padding":1,"children":[
        {"kind":"Text","children":["body"]},{"kind":"Box","display":"none","children":[{"kind":"Text","children":["hidden"]}]}]})).unwrap();
    let drawn = render(&view, 80);
    assert_eq!(drawn.len(), 3);
    assert_eq!(drawn[1].to_string(), " body   ");
}

#[test]
fn fixed_boxes_distribute_space_on_their_main_axis() {
    for (wire, expected) in [
        (
            serde_json::json!({"kind":"Box","width":3,"height":5,"justifyContent":"space-between","children":[{"kind":"Text","children":["a"]},{"kind":"Text","children":["b"]}]}),
            vec!["a  ", "   ", "   ", "   ", "b  "],
        ),
        (
            serde_json::json!({"kind":"Box","width":3,"height":5,"children":[{"kind":"Box","flexGrow":1,"children":[{"kind":"Text","children":["a"]}]},{"kind":"Text","children":["b"]}]}),
            vec!["a  ", "   ", "   ", "   ", "b  "],
        ),
        (
            serde_json::json!({"kind":"Box","width":3,"height":3,"flexDirection":"row","alignItems":"flex-end","children":[{"kind":"Text","children":["a"]}]}),
            vec!["   ", "   ", "a  "],
        ),
    ] {
        let view: View = serde_json::from_value(wire).unwrap();
        let actual: Vec<_> = render(&view, 3)
            .into_iter()
            .map(|line| line.to_string())
            .collect();
        assert_eq!(actual, expected);
    }
}
