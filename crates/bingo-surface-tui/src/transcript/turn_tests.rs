use super::*;
use crate::test_support::{assistant, folded};
use crate::{blocks::Blocks, test_support::*};
use bingo_sdk::{
    Event, TurnStatus,
    component::{StepState, Tier},
};

fn with_rows<R>(state: &SessionState, now: Now, draw: impl FnOnce(&Rows<'_>) -> R) -> R {
    let folds = Folds::new();
    let pictures = Default::default();
    let linked = Default::default();
    let rows = Rows::of(state, 80, &folds, &pictures, &linked, now);
    draw(&rows)
}

#[test]
fn idle_gap_returns_its_deadline_before_it_has_a_row() {
    let state = folded(vec![frame(1, started("trn_1"))]);
    let now = later(scene().1, 250);
    with_rows(&state, now, |rows| {
        let drawing = live::draw(&state, rows);
        assert!(drawing.lines.is_empty());
        assert_eq!(drawing.next, Some(now.instant + Duration::from_millis(250)));
        let mut blocks = Blocks::default();
        blocks.sync(&state, &Default::default(), rows);
        assert_eq!(blocks.next_change(0, 20), drawing.next);
    });
    with_rows(&state, later(now, 250), |rows| {
        assert!(
            live::draw(&state, rows).lines[0]
                .to_string()
                .contains("working")
        );
    });
}

#[test]
fn an_answer_is_whole_until_a_step_follows() {
    let mut state = folded(vec![
        frame(1, started("trn_1")),
        frame(
            2,
            Event::ItemCompleted {
                item: assistant("answer", "**All the words**", ItemStatus::Completed),
            },
        ),
    ]);
    with_rows(&state, scene().1, |rows| {
        assert!(!rows.followed.contains(&state.transcript.items()[0].id));
        let mut blocks = Blocks::default();
        let height = blocks.sync(&state, &Default::default(), rows);
        assert_eq!(blocks.tail(height).join("\n").trim(), "All the words");
    });
    state.apply(&frame(
        3,
        Event::ItemStarted {
            item: running_tool("step", "Inspect", "checking"),
        },
    ));
    with_rows(&state, scene().1, |rows| {
        let mut blocks = Blocks::default();
        blocks.sync(&state, &Default::default(), rows);
        let (from, to) = blocks.span(&bingo_sdk::ItemId::from_raw("answer")).unwrap();
        let lines = blocks.window(from, to - from);
        assert_eq!(lines[0].to_string().trim(), "**All the words**");
        assert!(lines[0].spans.iter().any(|s| {
            s.style
                .add_modifier
                .contains(ratatui::style::Modifier::ITALIC)
        }));
        assert!(blocks.next_change(from, to - from).is_none());
    });
}

#[test]
fn answers_show_their_streaming_and_stopped_ends() {
    for (status, ending) in [
        (ItemStatus::Running, "▍"),
        (ItemStatus::Interrupted, "− stopped"),
    ] {
        let state = folded(vec![
            frame(1, started("trn_1")),
            frame(
                2,
                Event::ItemStarted {
                    item: assistant("answer", "hello **world", status),
                },
            ),
        ]);
        with_rows(&state, scene().1, |rows| {
            let mut blocks = Blocks::default();
            let height = blocks.sync(&state, &Default::default(), rows);
            let text = blocks.tail(height).join("\n");
            assert!(text.trim().ends_with(ending), "{text}");
            assert!(!text.contains("**"), "{text}");
        });
    }
}

#[test]
fn turnless_calls_use_a_step_row() {
    let state = folded(vec![frame(
        1,
        Event::ItemCompleted {
            item: loose(tool(
                "step",
                "Inspect",
                serde_json::json!({"path":"target"}),
                None,
                ItemStatus::Completed,
            )),
        },
    )]);
    with_rows(&state, scene().1, |rows| {
        let mut blocks = Blocks::default();
        let height = blocks.sync(&state, &Default::default(), rows);
        let text = blocks.tail(height).join("\n");
        assert!(text.starts_with("   • inspect  target"), "{text}");
        assert!(!text.contains('⎿'));
    });
}

#[test]
fn step_motion_lands_and_reduced_motion_owes_nothing() {
    crate::theme::with(crate::painted::daylight(), || {
        let now = scene().1;
        let step = bingo_sdk::component::Step {
            mark: '!',
            reads: None,
            tier: Tier::Change,
            verb: "inspect".into(),
            target: "target".into(),
            state: StepState::Done,
            upshot: None,
            answer: None,
            elapsed_ms: None,
            why: None,
            landed_at: Some(now.wall.as_millisecond()),
        };
        let view = step.drawing(&crate::turns::marks(), 80);
        let draw = |at| crate::views::marked(&view, 80, &crate::views::Marks::at(at));
        assert!(draw(now).next.is_some());
        assert!(draw(later(now, 419)).next.is_some());
        assert!(draw(later(now, 420)).next.is_none());
        assert!(draw(still(now)).next.is_none());
        let mut asked = step;
        asked.state = StepState::Asked;
        let view = asked.drawing(&crate::turns::marks(), 80);
        assert!(
            crate::views::marked(&view, 80, &crate::views::Marks::at(now))
                .next
                .is_some()
        );
        assert!(
            crate::views::marked(&view, 80, &crate::views::Marks::at(still(now)))
                .next
                .is_none()
        );
    });
}

#[test]
fn only_the_final_failed_step_keeps_four_output_lines() {
    let failed = tool(
        "failed",
        "Inspect",
        serde_json::json!({"target":"target"}),
        Some(ToolOutput::text("old\none\ntwo\nerror: three\nfour")),
        ItemStatus::Failed,
    );
    let mut state = folded(vec![
        frame(1, started("trn_1")),
        frame(2, Event::ItemCompleted { item: failed }),
        frame(3, completed("trn_1", TurnStatus::Completed)),
    ]);
    let now = later(scene().1, 2000);
    with_rows(&state, now, |rows| {
        let mut blocks = Blocks::default();
        let height = blocks.sync(&state, &Default::default(), rows);
        let text = blocks.tail(height).join("\n");
        assert!(!text.contains("old"));
        assert!(text.contains("error: three") && text.contains("four"));
        let lines = blocks.window(0, height);
        assert!(lines[1].style.bg.is_none());
        assert!(
            lines[1].spans[0].style.bg.is_none(),
            "the well starts at column 5"
        );
    });
    state.apply(&frame(
        4,
        Event::ItemCompleted {
            item: tool(
                "recovery",
                "Inspect",
                serde_json::json!({}),
                Some(ToolOutput::text("fixed")),
                ItemStatus::Completed,
            ),
        },
    ));
    with_rows(&state, now, |rows| {
        let mut blocks = Blocks::default();
        let height = blocks.sync(&state, &Default::default(), rows);
        assert!(!blocks.tail(height).join("\n").contains("error: three"));
        assert_eq!(
            blocks.span(&bingo_sdk::ItemId::from_raw("failed")),
            Some((0, 1))
        );
    });
}

#[test]
fn offscreen_band_and_step_schedule_no_visible_motion() {
    let mut frames = vec![
        frame(1, started("trn_1")),
        frame(
            2,
            Event::ItemCompleted {
                item: user("prompt", "inspect"),
            },
        ),
        frame(
            3,
            Event::ItemStarted {
                item: running_tool("step", "Inspect", "checking"),
            },
        ),
    ];
    frames.extend((0..30).map(|at| {
        frame(
            at + 4,
            Event::ItemCompleted {
                item: loose(assistant(
                    &format!("later-{at}"),
                    "settled words",
                    ItemStatus::Completed,
                )),
            },
        )
    }));
    let state = folded(frames);
    with_rows(&state, later(scene().1, 2000), |rows| {
        let mut blocks = Blocks::default();
        let height = blocks.sync(&state, &Default::default(), rows);
        assert!(blocks.next_change(0, 3).is_some());
        assert!(blocks.next_change(height - 10, 10).is_none());
    });
}

#[test]
fn completion_sparks_keep_occupied_cells_and_end_at_seven_hundred_ms() {
    use ratatui::{buffer::Buffer, layout::Rect};
    crate::theme::with(crate::painted::daylight(), || {
        let (state, _) = crate::test_support::ember::story::scene("done");
        let mut now = scene().1;
        now.wall = state.last_turn.as_ref().unwrap().ended_at;
        let area = Rect::new(0, 0, 20, 8);
        let mut before = Buffer::empty(area);
        before[(1, 3)].set_symbol("x");
        let mut after = before.clone();
        let drawing = sparks::draw(&state, later(now, 350), &mut after, area, 5);
        assert!(drawing.next.is_some());
        assert!(drawing.lines.is_empty());
        let moved = crate::test_support::ember::moved(&before, &after);
        assert!(!moved.is_empty() && moved.len() <= 3);
        assert!(moved.iter().all(|&(x, y)| before[(x, y)].symbol() == " "));
        assert_eq!(after[(1, 3)].symbol(), "x");
        assert!(
            sparks::draw(&state, still(now), &mut after, area, 5)
                .next
                .is_none()
        );
        assert!(
            sparks::draw(&state, later(now, 700), &mut after, area, 5)
                .next
                .is_none()
        );
    });
}

#[test]
fn answered_and_skipped_questions_keep_their_receipts_on_the_step() {
    for answer in ["Route: Beta", "skipped · it decides"] {
        let receipt = crate::test_support::item(
            "receipt",
            ItemStatus::Completed,
            ItemBody::QuestionAnswer {
                asked_at: None,
                item: Some(bingo_sdk::ItemId::from_raw("question")),
                interaction: bingo_sdk::InteractionId::from_raw("ask"),
                question: "Which route?".into(),
                answer: answer.into(),
            },
        );
        let state = folded(vec![
            frame(1, started("trn_1")),
            frame(
                2,
                Event::ItemCompleted {
                    item: tool(
                        "question",
                        "Question",
                        serde_json::json!({}),
                        None,
                        ItemStatus::Completed,
                    ),
                },
            ),
            frame(3, Event::ItemCompleted { item: receipt }),
            frame(4, completed("trn_1", TurnStatus::Completed)),
        ]);
        with_rows(&state, later(scene().1, 2000), |rows| {
            let mut blocks = Blocks::default();
            let height = blocks.sync(&state, &Default::default(), rows);
            let text = blocks.tail(height).join("\n");
            assert_eq!(text.matches(answer).count(), 1, "{text}");
            assert!(!text.contains("you:"), "{text}");
        });
    }
}

#[test]
fn a_principal_keeps_their_own_band_in_a_turn() {
    let state = folded(vec![
        frame(1, started("trn_1")),
        frame(
            2,
            Event::ItemCompleted {
                item: user("own", "hello"),
            },
        ),
        frame(
            3,
            Event::ItemCompleted {
                item: delivered("peer", "channel", Some("mei"), "look at the deploy?"),
            },
        ),
    ]);
    with_rows(&state, later(scene().1, 2000), |rows| {
        let mut blocks = Blocks::default();
        let height = blocks.sync(&state, &Default::default(), rows);
        let text = blocks.tail(height).join("\n");
        assert!(text.contains("mei  look at the deploy?"), "{text}");
        assert!(!text.contains("steered"), "{text}");
        assert_eq!(text.matches("2s").count(), 1, "{text}");
    });
}

#[test]
fn a_principal_starting_the_turn_owns_its_light_and_clock() {
    let state = folded(vec![
        frame(1, started("trn_1")),
        frame(
            2,
            Event::ItemCompleted {
                item: delivered("peer", "channel", Some("mei"), "look at the deploy?"),
            },
        ),
    ]);
    with_rows(&state, later(scene().1, 2000), |rows| {
        let item = state.transcript.items().first().unwrap();
        let drawn = band::draw(item, rows);
        assert!(
            drawn.lines[0]
                .to_string()
                .contains("mei  look at the deploy?")
        );
        assert!(drawn.lines[0].to_string().ends_with("2s  "));
        assert_eq!(drawn.lines[0].to_string().chars().next(), Some('▍'));
        assert!(drawn.next.is_some());
    });
}

#[test]
fn narration_followed_by_a_step_allows_working() {
    for status in [ItemStatus::Running, ItemStatus::Completed] {
        let state = folded(vec![
            frame(1, started("trn_1")),
            frame(
                2,
                Event::ItemCompleted {
                    item: assistant("reply", "I checked it.", status),
                },
            ),
            frame(
                3,
                Event::ItemCompleted {
                    item: tool(
                        "step",
                        "Inspect",
                        serde_json::json!({}),
                        None,
                        ItemStatus::Completed,
                    ),
                },
            ),
        ]);
        with_rows(&state, later(scene().1, 2000), |rows| {
            assert!(
                live::draw(&state, rows).lines[0]
                    .to_string()
                    .contains("working")
            );
        });
    }
}

#[test]
fn a_running_answer_has_arrival_ink_only_in_an_unpaused_turn() {
    crate::theme::with(crate::painted::daylight(), || {
        let mut state = folded(vec![
            frame(1, started("trn_1")),
            frame(
                2,
                Event::ItemStarted {
                    item: assistant("reply", "arriving words", ItemStatus::Running),
                },
            ),
        ]);
        let mut blocks = Blocks::default();
        let mut draw = |state: &SessionState| {
            with_rows(state, scene().1, |rows| {
                blocks.sync(state, &Default::default(), rows);
                let (from, to) = blocks.span(&bingo_sdk::ItemId::from_raw("reply")).unwrap();
                blocks.window(from, to - from)
            })
        };
        let live = draw(&state);
        assert!(live[0].to_string().ends_with('▍'));
        assert!(
            live[0]
                .spans
                .iter()
                .any(|span| span.content.contains('s') && span.style.fg != theme::text().fg),
            "{live:?}"
        );
        state.apply(&frame(3, opened(permission(None, None))));
        let paused = draw(&state);
        assert_eq!(paused[0].to_string(), "   arriving words");
        assert!(
            paused[0]
                .spans
                .iter()
                .filter(|span| !span.content.trim().is_empty())
                .all(|span| span.style.fg == theme::text().fg)
        );
        state.apply(&frame(4, completed("trn_1", TurnStatus::Completed)));
        assert_eq!(draw(&state)[0].to_string(), "   arriving words");
    });
}

#[test]
fn a_named_band_inside_a_turn_has_no_clock_revision() {
    let state = folded(vec![
        frame(1, started("trn_1")),
        frame(
            2,
            Event::ItemCompleted {
                item: user("prompt", "check it"),
            },
        ),
        frame(
            3,
            Event::ItemCompleted {
                item: delivered("peer", "channel", Some("mei"), "also this"),
            },
        ),
    ]);
    let peer = &state.transcript.items()[1];
    with_rows(&state, later(scene().1, 2000), |rows| {
        assert!(band::revision(peer, rows).is_none());
        assert!(band::draw(peer, rows).next.is_none());
    });
}

#[test]
fn a_turnless_finished_call_has_no_output_preview() {
    let state = folded(vec![frame(
        1,
        Event::ItemCompleted {
            item: loose(tool(
                "step",
                "Inspect",
                serde_json::json!({"path":"target"}),
                Some(ToolOutput::text("private detail\n".repeat(9))),
                ItemStatus::Completed,
            )),
        },
    )]);
    with_rows(&state, scene().1, |rows| {
        let mut blocks = Blocks::default();
        blocks.sync(&state, &Default::default(), rows);
        let (from, to) = blocks.span(&bingo_sdk::ItemId::from_raw("step")).unwrap();
        assert_eq!(to - from, 1);
        assert!(
            !blocks.window(from, to - from)[0]
                .to_string()
                .contains("detail")
        );
    });
}

#[test]
fn quiet_and_opened_calls_place_the_same_display_body() {
    let display = bingo_sdk::View::text("a displayed body");
    let draw = |quiet| {
        let mut call = loose(tool(
            "step",
            "Inspect",
            serde_json::json!({}),
            Some(ToolOutput {
                display: Some(display.clone()),
                ..ToolOutput::text("")
            }),
            ItemStatus::Completed,
        ));
        if let ItemBody::ToolCall { quiet: flag, .. } = &mut call.body {
            *flag = quiet;
        }
        let state = folded(vec![frame(1, Event::ItemCompleted { item: call })]);
        with_rows(&state, scene().1, |rows| {
            let folds = [(bingo_sdk::ItemId::from_raw("step"), Fold::Open)]
                .into_iter()
                .collect();
            let rows = Rows {
                folds: &folds,
                ..rows.clone()
            };
            let mut blocks = Blocks::default();
            let height = blocks.sync(&state, &Default::default(), &rows);
            blocks
                .tail(height)
                .into_iter()
                .find(|line| line.contains("a displayed body"))
                .unwrap()
        })
    };
    assert_eq!(draw(false), "     a displayed body");
    assert_eq!(draw(true), draw(false));
}

#[test]
fn narration_respects_the_same_measure_as_the_answer() {
    let state = folded(vec![
        frame(1, started("trn_1")),
        frame(
            2,
            Event::ItemCompleted {
                item: assistant("reply", &"word ".repeat(15), ItemStatus::Completed),
            },
        ),
        frame(
            3,
            Event::ItemStarted {
                item: running_tool("step", "Inspect", "checking"),
            },
        ),
    ]);
    with_rows(&state, scene().1, |rows| {
        let rows = rows.clone().measuring(Some(30));
        let mut blocks = Blocks::default();
        blocks.sync(&state, &Default::default(), &rows);
        let (from, to) = blocks.span(&bingo_sdk::ItemId::from_raw("reply")).unwrap();
        let lines = blocks.window(from, to - from);
        assert!(lines.len() >= 3);
        assert!(lines.iter().all(|line| line.width() <= 30));
    });
}

#[test]
fn an_idle_rows_deadline_is_the_declared_motion_deadline() {
    let state = folded(vec![frame(1, started("trn_1"))]);
    let now = later(scene().1, 719);
    crate::theme::with(crate::painted::no_colour(), || {
        with_rows(&state, now, |rows| {
            assert_eq!(
                live::draw(&state, rows).next,
                Some(crate::spinner::next(now))
            );
        });
    });
}

#[test]
fn an_idle_turn_waiting_for_a_person_does_not_say_working() {
    let state = folded(vec![
        frame(1, started("trn_1")),
        frame(2, opened(permission(None, None))),
    ]);
    with_rows(&state, later(scene().1, 2000), |rows| {
        let drawn = live::draw(&state, rows);
        assert!(drawn.lines.is_empty());
        assert!(drawn.next.is_none());
    });
}

#[test]
fn a_tools_picture_stays_under_its_step_without_opening_detail() {
    let output = ToolOutput {
        parts: vec![bingo_sdk::ContentPart::Image(bingo_pictures::testing::png(
            8, 8,
        ))],
        display: None,
        is_error: false,
        upshot: None,
    };
    let state = folded(vec![
        frame(1, started("trn_1")),
        frame(
            2,
            Event::ItemCompleted {
                item: tool(
                    "picture",
                    "Inspect",
                    serde_json::json!({}),
                    Some(output),
                    ItemStatus::Completed,
                ),
            },
        ),
    ]);
    crate::graphics::with(crate::graphics::Graphics::Off, || {
        with_rows(&state, scene().1, |rows| {
            let mut blocks = Blocks::default();
            blocks.sync(&state, &Default::default(), rows);
            let (from, to) = blocks
                .span(&bingo_sdk::ItemId::from_raw("picture"))
                .unwrap();
            let lines = blocks.window(from, to - from);
            assert_eq!(lines.len(), 2);
            assert_eq!(lines[1].to_string(), "     [image: image/png]");
        });
    });
}
