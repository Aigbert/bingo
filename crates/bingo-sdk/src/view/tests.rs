use super::*;

fn action(name: &str) -> Action {
    Action {
        name: name.into(),
        args: serde_json::Value::Null,
    }
}

/// Every node once: the wire shape and the fold are the contract.
fn every_node() -> Vec<(View, &'static str)> {
    vec![
        (View::text("plain"), "plain"),
        (
            View::Markdown {
                text: "# Title\n\nbody".into(),
                dim_color: false,
            },
            "# Title\n\nbody",
        ),
        (
            View::Code {
                language: Some("rust".into()),
                source: "fn main() {}".into(),
            },
            "fn main() {}",
        ),
        (
            View::Diff {
                unified: "@@ -1 +1 @@\n-a\n+b\n".into(),
            },
            "@@ -1 +1 @@\n-a\n+b\n",
        ),
        (
            View::List {
                items: vec!["one".into(), "two".into()],
            },
            "- one\n- two",
        ),
        (
            View::Table {
                headers: vec!["name".into(), "state".into()],
                rows: vec![vec!["reviewer".into(), "running".into()]],
            },
            "name · state\nreviewer · running",
        ),
        (
            View::KeyValue {
                rows: vec![("model".into(), "gpt-5.4".into())],
            },
            "model: gpt-5.4",
        ),
        (
            View::Progress {
                value: 24,
                total: Some(30),
                label: Some("cargo test".into()),
            },
            "cargo test 80 %",
        ),
        (
            View::Badge {
                text: "needs you".into(),
                tone: Tone::Attention,
            },
            "[needs you]",
        ),
        (
            View::Tree {
                nodes: vec![TreeNode {
                    label: "project".into(),
                    badge: None,
                    tone: Tone::Neutral,
                    children: vec![TreeNode {
                        label: "reviewer".into(),
                        badge: Some("waiting".into()),
                        tone: Tone::Attention,
                        children: Vec::new(),
                    }],
                }],
            },
            "project\n  reviewer [waiting]",
        ),
        (View::column(vec![View::text("a"), View::text("b")]), "a\nb"),
        (
            View::columns(vec![View::text("left"), View::text("right")]),
            "left\nright",
        ),
        (
            View::Panel {
                title: "Board".into(),
                child: Box::new(View::text("empty")),
            },
            "Board\nempty",
        ),
        (
            View::Actions {
                items: vec![
                    ActionItem {
                        meaning: None,
                        label: "Approve".into(),
                        action: action("board.tick"),
                        key: Some('1'),
                    },
                    ActionItem {
                        meaning: None,
                        label: "Next".into(),
                        action: action("board.next"),
                        key: None,
                    },
                ],
            },
            "[Approve] [Next]",
        ),
    ]
}

/// The committed shape of the vocabulary, and the fence the catch-all is
/// built against (ADR-0038 §2): a known kind reads back as itself and
/// writes the same bytes it was read from. This snapshot moving means a
/// reader that predates the change stopped understanding a word it knew.
#[test]
fn every_node_has_a_wire_shape_and_round_trips() {
    let views: Vec<View> = every_node().into_iter().map(|(v, _)| v).collect();
    insta::with_settings!({snapshot_path => "../snapshots"}, {
        insta::assert_json_snapshot!("views", views);
    });
    for view in &views {
        let json = serde_json::to_string(view).unwrap();
        let read: View = serde_json::from_str(&json).unwrap();
        assert_eq!(&read, view);
        assert_eq!(serde_json::to_string(&read).unwrap(), json);
    }
}

#[test]
fn every_node_folds_to_text() {
    for (view, fold) in every_node() {
        assert_eq!(view.fold(), fold, "{view:?}");
    }
}

fn a_table() -> View {
    View::Table {
        headers: vec!["name".into(), "state".into()],
        rows: vec![
            vec!["reviewer".into(), "running".into()],
            vec!["scout".into(), "idle".into()],
        ],
    }
}

/// The one node the two walks differ on: what a markdown surface draws a
/// table from is not what ` · ` spells.
#[test]
fn a_table_is_a_table_to_a_surface_that_draws_markdown() {
    assert_eq!(
        a_table().fold_markdown(),
        "| name | state |\n| --- | --- |\n| reviewer | running |\n| scout | idle |"
    );
    assert_eq!(
        a_table().fold(),
        "name · state\nreviewer · running\nscout · idle"
    );
}

/// A header row with nothing under it is still a table: the header is what
/// says which emptiness this is, and a surface that draws one draws it.
#[test]
fn a_table_with_no_rows_keeps_its_header() {
    let empty = View::Table {
        headers: vec!["server".into(), "status".into()],
        rows: Vec::new(),
    };
    assert_eq!(empty.fold_markdown(), "| server | status |\n| --- | --- |");
}

/// A cell carrying the separator or a line break would make one row into
/// two, so each is written as the markdown that stands for it.
#[test]
fn a_cell_that_would_break_its_row_is_written_as_markdown() {
    let awkward = View::Table {
        headers: vec!["a".into()],
        rows: vec![vec!["x | y".into()], vec!["x\ny".into()]],
    };
    assert_eq!(
        awkward.fold_markdown(),
        "| a |\n| --- |\n| x \\| y |\n| x<br>y |"
    );
}

#[test]
fn every_line_ending_stays_inside_its_table_cell() {
    for ending in ["\n", "\r\n", "\r"] {
        let table = View::Table {
            headers: vec![format!("tool{ending}name"), "description".into()],
            rows: vec![vec!["Read".into(), format!("first{ending}second")]],
        };
        assert_eq!(
            table.fold_markdown(),
            "| tool<br>name | description |\n| --- | --- |\n| Read | first<br>second |"
        );
    }
}

/// The walk is the whole view, not the top of it: a table inside a stack
/// or a panel is drawn by the same rule as one on its own.
#[test]
fn a_table_inside_another_node_is_drawn_by_the_same_rule() {
    let nested = View::Panel {
        title: "memories".into(),
        child: Box::new(a_table()),
    };
    assert_eq!(
        nested.fold_markdown(),
        "memories\n\n| name | state |\n| --- | --- |\n| reviewer | running |\n| scout | idle |"
    );
    assert_eq!(
        nested.fold(),
        "memories\nname · state\nreviewer · running\nscout · idle",
        "the joined walk is unchanged: nothing there needs a blank line"
    );
    let stacked = View::column(vec![View::text("schedules: standby"), a_table()]);
    assert_eq!(
        stacked.fold_markdown(),
        "schedules: standby\n\n| name | state |\n| --- | --- |\n\
             | reviewer | running |\n| scout | idle |",
        "prose then a table: without the blank line the prose is another row"
    );
}

/// The other half of the fence: a word this sdk has no name for degrades
/// (ADR-0038 §2), but a known word spelled wrong is still a mistake, and
/// so is a node with no word at all.
#[test]
fn a_known_kind_that_is_malformed_is_still_an_error() {
    for json in [
        r#"{"kind":"text"}"#,
        r#"{"kind":"table","headers":[]}"#,
        r#"{"kind":"progress","value":"a lot"}"#,
        r#"{"text":"no kind at all"}"#,
        r#"{"kind":42,"text":"a kind is a word"}"#,
        r#""not an object at all""#,
    ] {
        assert!(serde_json::from_str::<View>(json).is_err(), "{json}");
    }
}

#[test]
fn an_unbounded_progress_folds_to_its_count() {
    let view = View::Progress {
        value: 7,
        total: None,
        label: None,
    };
    assert_eq!(view.fold(), "7");
}

fn sparkline() -> View {
    View::Custom {
        kind: "demo.sparkline".into(),
        data: serde_json::json!({"points": [3, 5, 8, 13]}),
        fold: "3 5 8 13".into(),
    }
}

/// The list the catch-all consults is the vocabulary itself.
#[test]
fn the_known_kinds_are_the_vocabulary() {
    let schema = serde_json::to_value(schemars::schema_for!(View)).unwrap();
    let kinds: Vec<&str> = schema["oneOf"]
        .as_array()
        .expect("one shape per node")
        .iter()
        .map(|node| {
            node["properties"]["kind"]["const"]
                .as_str()
                .expect("a kind")
        })
        .collect();
    assert_eq!(kinds, KNOWN);
}

/// The spelling a custom node rides under: the tag says `custom`, which
/// this sdk knows, and the plugin's own word sits beside it. That is what
/// keeps a custom node from being caught as unknown by its own reader and
/// wrapped a second time.
#[test]
fn a_custom_node_writes_the_reserved_shape_and_reads_back_as_itself() {
    let json = serde_json::to_value(sparkline()).unwrap();
    assert_eq!(
        json,
        serde_json::json!({
            "kind": "custom",
            "customKind": "demo.sparkline",
            "data": {"points": [3, 5, 8, 13]},
            "fold": "3 5 8 13",
        })
    );
    assert_eq!(serde_json::from_value::<View>(json).unwrap(), sparkline());
    assert_eq!(sparkline().fold(), "3 5 8 13");
}

/// The door ADR-0038 §2 opens: a word from a newer speaker is text here,
/// not an error, and everything it said is still in `data`.
#[test]
fn a_kind_this_sdk_has_never_heard_of_lands_in_custom_whole() {
    let from_the_future = serde_json::json!({
        "kind": "chart.candles",
        "series": [1, 2],
        "fold": "AAPL 1 2",
    });
    assert_eq!(
        serde_json::from_value::<View>(from_the_future.clone()).unwrap(),
        View::Custom {
            kind: "chart.candles".into(),
            data: from_the_future,
            fold: "AAPL 1 2".into(),
        }
    );
}

/// A newer speaker that forgot its fold still says which word it was.
#[test]
fn an_unknown_kind_with_no_fold_folds_to_its_name() {
    let view: View = serde_json::from_value(serde_json::json!({"kind": "chart.candles"}))
        .expect("a word this sdk has not learned is text, not an error");
    assert_eq!(view.fold(), "[chart.candles]");
}

/// Both origins of a `Custom` survive a round trip, which is what lets a
/// surface pass a node it cannot draw on to one that can.
#[test]
fn a_caught_node_round_trips_without_nesting_itself_again() {
    let caught: View = serde_json::from_value(serde_json::json!({
        "kind": "chart.candles",
        "series": [1, 2],
    }))
    .unwrap();
    let written = serde_json::to_string(&caught).unwrap();
    let read: View = serde_json::from_str(&written).unwrap();
    assert_eq!(read, caught);
    assert_eq!(serde_json::to_string(&read).unwrap(), written);
}

/// A node from the future nested inside a node this sdk knows: the parse
/// degrades where the unknown word is and nowhere else.
#[test]
fn an_unknown_node_inside_a_known_one_folds_in_place() {
    let view: View = serde_json::from_value(serde_json::json!({
        "kind": "panel",
        "title": "Board",
        "child": {"kind": "chart.candles", "fold": "AAPL 1 2"},
    }))
    .expect("a panel around a word from the future is still a panel");
    assert_eq!(view.fold(), "Board\nAAPL 1 2");
}

#[test]
fn round2_action_meaning_is_optional_and_round_trips() {
    let old = serde_json::json!({"label":"plan", "action":{"name":"permission","args":"plan"}});
    let item: super::ActionItem = serde_json::from_value(old.clone()).unwrap();
    assert_eq!(serde_json::to_value(item).unwrap(), old);
    let mut wire = old;
    wire["meaning"] = serde_json::json!("a policy-owned sentence");
    let item: super::ActionItem = serde_json::from_value(wire.clone()).unwrap();
    assert_eq!(serde_json::to_value(item).unwrap(), wire);
}

#[test]
fn round2_keybar_has_only_pairs() {
    let wire = serde_json::to_value(crate::component::KeyBarProps::default()).unwrap();
    assert_eq!(wire, serde_json::json!({"pairs":[]}));
}
