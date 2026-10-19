use crate::test_support::ember::{Band, Capture, assert_regions, moved};
use crate::test_support::*;
use crate::ui::Ui;
use bingo_sdk::{Event, ItemStatus, Level, SessionState, TurnStatus};
use serde_json::json;

fn apply(state: &mut SessionState, at: i64, event: Event) {
    let mut frame = frame(state.seq.0 + 1, event);
    frame.ts = jiff::Timestamp::from_millisecond(crate::test_support::ember::CLOCK_ORIGIN_MS + at)
        .unwrap();
    state.apply(&frame);
}

/// The prompt and step sequence from p4b-data.js, independent of stream drawing.
fn past(state: &mut SessionState, turns: usize) {
    for (turn, prompt, answer) in [
        (
            "t1",
            "看一下 session/queue.rs 里的 steer 逻辑，为什么排队的消息有时候会被吞掉",
            "问题不在 drain()，在 reset()：每一轮开始时，它把 queue 整个清空了。",
        ),
        (
            "t2",
            "修好它，加一个回归测试",
            "修好了。reset() 只重置 steer，排队的消息留给下一轮。",
        ),
    ]
    .into_iter()
    .take(turns)
    {
        apply(state, -10_000_000, started(turn));
        apply(
            state,
            -10_000_000,
            Event::ItemCompleted {
                item: user(&format!("{turn}-prompt"), prompt),
            },
        );
        apply(
            state,
            -9_990_000,
            Event::ItemCompleted {
                item: assistant(&format!("{turn}-answer"), answer, ItemStatus::Completed),
            },
        );
        apply(state, -9_990_000, completed(turn, TurnStatus::Completed));
    }
}

fn fixture(capture: &Capture) -> (SessionState, Ui) {
    let (mut ui, _) = scene();
    let mut state = state();
    let mut summary = summary();
    summary.model = Some("opus-5.5".into());
    summary.effort = Some(bingo_sdk::Effort::High);
    summary.title = Some("steer 吞掉排队消息".into());
    apply(&mut state, -10_000_001, Event::SessionUpdated { summary });
    if capture.entry.scene == "welcome" {
        ui.welcome.resume = Some(state.summary.clone());
        return (state, ui);
    }
    let live = matches!(capture.entry.scene.as_str(), "live" | "narrow");
    past(&mut state, if live { 1 } else { 2 });
    if live {
        apply(&mut state, -2600, permission_view("acceptEdits"));
        apply(&mut state, -2600, started("live"));
        apply(
            &mut state,
            -2600,
            Event::ItemCompleted {
                item: user("live-prompt", "修好它，加一个回归测试"),
            },
        );
        for (id, name, path, at) in [
            (
                "read",
                "Read",
                "crates/bingo-core/src/session/queue.rs",
                -1700,
            ),
            ("find", "Glob", "crates/bingo-core/tests/*.rs", -1400),
            (
                "edit",
                "Edit",
                "crates/bingo-core/src/session/queue.rs",
                -1100,
            ),
            (
                "write",
                "Write",
                "crates/bingo-core/tests/queue_steer.rs",
                -550,
            ),
        ] {
            apply(
                &mut state,
                at,
                Event::ItemCompleted {
                    item: tool(id, name, json!({"path":path}), None, ItemStatus::Completed),
                },
            );
        }
        apply(
            &mut state,
            0,
            Event::ItemStarted {
                item: running_tool(
                    "bash",
                    "Bash",
                    "Compiling bingo-core v0.6.3 (crates/bingo-core)",
                ),
            },
        );
        let row: bingo_sdk::slot::Slot = serde_json::from_value(json!({"kind":"row","item":"bash","mark":null,"title":null,"badge":null,"body":null,"actions":[{"label":"background","key":"b","action":{"name":"bash.promote","args":{}}}]})).unwrap();
        apply(
            &mut state,
            0,
            extended(
                "bingo.bash",
                "running",
                bingo_sdk::slot::with(json!({}), [row]),
            ),
        );
    }
    if capture.entry.scene == "mode" {
        let (value, at) = if capture.entry.file == "mode-later" {
            ("bypassPermissions", 3400)
        } else {
            ("plan", 2000)
        };
        apply(&mut state, at, permission_view(value));
        let now = capture.now();
        ui.notify_mode(
            now.instant - std::time::Duration::from_millis((capture.entry.moment_ms - at) as u64),
        );
    }
    (state, ui)
}

#[test]
fn m193_capture_dock_and_footer_bands() {
    for name in [
        "live",
        "done",
        "mode",
        "mode-later",
        "reduced-mode",
        "reduced-live",
        "ascii-live",
        "ascii-done",
        "narrow",
        "small-done",
        "welcome",
    ] {
        let capture = Capture::load(name);
        let (state, mut ui) = fixture(&capture);
        capture.render(|| {
            let size = capture.size();
            let screen = drawn(
                size.width,
                size.height,
                &solo(&state),
                &mut ui,
                capture.now(),
            );
            assert_regions(&capture, Band::Dock, screen.buffer(), &ui.shown.regions);
            assert_regions(&capture, Band::Footer, screen.buffer(), &ui.shown.regions);
        });
    }
}

fn edge(
    state: &SessionState,
    ui: &Ui,
    now: crate::clock::Now,
) -> (ratatui::buffer::Buffer, Option<std::time::Instant>) {
    use ratatui::{
        buffer::Buffer,
        layout::Rect,
        widgets::{Paragraph, Widget},
    };
    let drawing = crate::dock::edge(&solo(state), ui, 100, now);
    let mut buffer = Buffer::empty(Rect::new(0, 0, 100, 1));
    Paragraph::new(drawing.lines).render(buffer.area, &mut buffer);
    (buffer, drawing.next)
}

#[test]
fn m193_edge_comet_end_mode_and_reduced_have_deadlines() {
    let capture = Capture::load("live");
    let (mut state, ui) = fixture(&capture);
    capture.render(|| {
        let now = capture.now();
        let a = edge(&state, &ui, now);
        let b = edge(&state, &ui, later(now, 550));
        let c = edge(&state, &ui, later(now, 1100));
        assert!(!moved(&a.0, &b.0).is_empty());
        assert!(!moved(&b.0, &c.0).is_empty());
        assert!(a.1.is_some());
        let reduced = edge(&state, &ui, still(now));
        assert!(reduced.1.is_none());
        assert!(moved(&reduced.0, &edge(&state, &ui, still(later(now, 550))).0).is_empty());
        apply(
            &mut state,
            capture.entry.moment_ms,
            completed("live", TurnStatus::Completed),
        );
        let a = edge(&state, &ui, now);
        let b = edge(&state, &ui, later(now, 550));
        let c = edge(&state, &ui, later(now, 1100));
        assert!(!moved(&a.0, &b.0).is_empty());
        assert!(c.0.content().iter().all(|cell| cell.symbol() == " "));
        assert!(c.1.is_none());
    });
    let capture = Capture::load("mode");
    let (state, ui) = fixture(&capture);
    assert!(edge(&state, &ui, capture.now()).1.is_some());
    assert!(edge(&state, &ui, later(capture.now(), 900)).1.is_none());
}

#[test]
fn m193_narrow_footer_drops_keys_then_facts_and_keeps_the_model() {
    let (ui, now) = scene();
    let state = folded(vec![frame(1, permission_view("plan"))]);
    let tree = solo(&state);
    let at = |width| {
        super::draw(&tree, &ui, &mut Default::default(), width, still(now)).lines[0].to_string()
    };
    assert!(at(80).contains("/ commands"));
    assert!(!at(25).contains("? keys"));
    assert!(at(25).contains("plan"));
    assert!(at(12).contains("fake-1"));
    assert!(!at(12).contains("plan"));
}

#[test]
fn ambient_deadlines_survive_clipping_but_not_toast_replacement() {
    let tree = folded_tree(busy_child("reviewer"));
    let (mut ui, now) = scene();
    let draw = |ui: &Ui, width, now| super::draw(&tree, ui, &mut Default::default(), width, now);
    let next = Some(now.instant + crate::clock::FRAME);
    assert_eq!(draw(&ui, 80, now).next, next);
    assert_eq!(draw(&ui, 12, now).next, next);
    ui.notify(Level::Info, "saved", now.instant);
    assert_eq!(
        draw(&ui, 80, later(now, 200)).next,
        Some(now.instant + std::time::Duration::from_millis(2400))
    );
}

#[test]
fn a_mode_shaped_sentence_is_only_a_plain_notice() {
    let (mut ui, now) = scene();
    let state = folded(vec![frame(1, permission_view("plan"))]);
    let text = "permission mode: plan";
    ui.notify(Level::Info, text, now.instant);
    let lines = super::draw(
        &solo(&state),
        &ui,
        &mut Default::default(),
        100,
        later(now, 200),
    );
    assert!(lines.lines[0].to_string().contains(text));
    assert!(
        edge(&state, &ui, now)
            .0
            .content()
            .iter()
            .all(|c| c.symbol() == " ")
    );
}

#[test]
fn the_mode_toast_reads_the_placed_meaning() {
    let (mut ui, now) = scene();
    let slot = json!({"kind":"mode","current":"plan","modes":[{"label":"plan","meaning":"the policy changed this sentence","action":{"name":"permission","args":"plan"}}]});
    let state = folded(vec![frame(
        1,
        extended("policy", "mode", json!({"slots":[slot]})),
    )]);
    ui.notify_mode(now.instant);
    let drawn = super::draw(
        &solo(&state),
        &ui,
        &mut Default::default(),
        100,
        later(now, 200),
    );
    assert!(
        drawn.lines[0]
            .to_string()
            .contains("the policy changed this sentence")
    );
}

#[test]
fn the_idle_caret_rests_and_carries_no_deadline() {
    let (mut ui, now) = scene();
    let tree = solo(&state());
    let rest = later(now, 10_001);
    let screen = drawn(80, 24, &tree, &mut ui, rest);
    let y = ui.shown.regions.composer.y + 1;
    assert_eq!(
        screen.buffer()[(3, y)].fg,
        crate::theme::caret_ink(still(rest), "glow", "you")
            .unwrap()
            .fg
            .unwrap()
    );
    assert!(ui.shown.deadline().is_none());
}

#[test]
fn reduced_activity_wakes_for_its_first_row() {
    let (mut ui, now) = scene();
    let tree = solo(&folded(vec![frame(1, started("turn"))]));
    draw_tree(80, 24, &tree, &mut ui, still(now));
    assert_eq!(ui.shown.regions.activity.height, 0);
    assert_eq!(
        ui.shown.deadline(),
        Some(now.instant + std::time::Duration::from_millis(500))
    );
    draw_tree(80, 24, &tree, &mut ui, still(later(now, 500)));
    assert_eq!(ui.shown.regions.activity.height, 0);
    assert!(draw_tree(80, 24, &tree, &mut ui, still(later(now, 500))).contains("working"));
}

#[test]
fn the_latest_edge_flash_wins() {
    let (mut ui, now) = scene();
    let state = folded(vec![
        frame(1, permission_view("plan")),
        frame(2, started("turn")),
        frame(3, completed("turn", TurnStatus::Completed)),
    ]);
    ui.notify_mode(now.instant - std::time::Duration::from_millis(100));
    let newer_end = crate::dock::edge(&solo(&state), &ui, 80, still(now));
    assert_eq!(
        newer_end.lines[0].spans[0].style,
        crate::theme::blended("good", "you", 0.2).unwrap()
    );
    ui.notify_mode(later(now, 100).instant);
    let newer_mode = crate::dock::edge(&solo(&state), &ui, 80, still(later(now, 100)));
    assert_eq!(
        newer_mode.lines[0].spans[0].style,
        crate::theme::blended("plan", "you", 0.2).unwrap()
    );
}

#[test]
fn a_countdown_wakes_only_when_its_words_change() {
    use std::time::Duration;
    let epsilon = Duration::from_nanos(1);
    for (remaining, change) in [(125, 5), (3605, 5), (90_000, 3600), (59, 0)] {
        assert_eq!(
            super::countdown_change(Duration::from_secs(remaining)),
            Duration::from_secs(change) + epsilon
        );
    }
}

#[test]
fn needs_you_replaces_crew_lamps_before_the_model() {
    let tree = folded_tree(vec![
        child_frame(1, announced("reviewer")),
        child_frame(2, opened(child_permission())),
    ]);
    let (ui, now) = scene();
    let line =
        crate::footer::draw(&tree, &ui, &mut Default::default(), 120, now).lines[0].to_string();
    assert!(
        line.trim_start().starts_with("◆ reviewer needs you"),
        "{line}"
    );
    assert!(!line.contains("agents"));
    assert!(!line.contains("ctrl+g"));
}

#[test]
fn needs_you_names_one_two_or_many_with_their_inks() {
    for count in 1..=3 {
        let mut frames = Vec::new();
        for (index, name) in ["reviewer", "tester", "writer"]
            .into_iter()
            .take(count)
            .enumerate()
        {
            let id = bingo_sdk::SessionId::from_raw(format!("child-{index}"));
            let mut summary = child_summary(name);
            summary.id = id.clone();
            for (seq, event) in [
                (1, Event::SessionUpdated { summary }),
                (2, opened(child_permission())),
            ] {
                let mut next = frame(seq, event);
                next.session = id.clone();
                frames.push(next);
            }
        }
        let tree = folded_tree(frames);
        let drawn = super::needs_you(&tree);
        let line = &drawn.lines[0];
        let expected = [
            "◆ reviewer needs you",
            "◆ reviewer, tester need you",
            "◆ 3 need you",
        ][count - 1];
        assert_eq!(line.to_string(), expected);
        if count > 2 {
            let count = line
                .spans
                .iter()
                .find(|span| span.content == "3")
                .expect("the count");
            assert_eq!(count.style, super::ink("amber"));
        }
        let faces = crate::face::Faces::of(&tree);
        for span in &line.spans {
            if ["reviewer", "tester"].contains(&span.content.as_ref()) {
                assert_eq!(span.style, super::ink(faces.who(&span.content)));
            }
        }
    }
}

#[test]
fn needs_you_calls_the_main_session_bingo_in_glow() {
    let mut tree = folded_tree(vec![
        child_frame(1, announced("reviewer")),
        frame(1, opened(permission(None, None))),
    ]);
    tree.show(&child_id());
    let drawn = super::needs_you(&tree);
    let line = &drawn.lines[0];
    assert_eq!(line.to_string(), "◆ bingo needs you");
    assert_eq!(
        line.spans
            .iter()
            .find(|span| span.content == "bingo")
            .unwrap()
            .style,
        super::ink("glow")
    );
}

#[test]
fn ember_full_frames_match_the_prototype() {
    for name in [
        "live",
        "done",
        "ascii-live",
        "ascii-done",
        "reduced-live",
        "small-done",
        "mode",
        "mode-later",
    ] {
        let capture = Capture::load(name).spinners(if matches!(name, "live" | "ascii-live") {
            &[(25, 3)]
        } else {
            &[]
        });
        let (state, mut ui) = full_fixture(&capture);
        capture.render(|| {
            let size = capture.size();
            let screen = drawn(
                size.width,
                size.height,
                &solo(&state),
                &mut ui,
                capture.now(),
            );
            assert_regions(&capture, Band::Frame, screen.buffer(), &ui.shown.regions);
        });
    }
}

fn full_fixture(capture: &Capture) -> (SessionState, Ui) {
    let (dock, mut ui) = fixture(capture);
    let (mut state, _) = crate::test_support::ember::story::scene(&capture.entry.file);
    let slots = bingo_sdk::slot::placed(&dock);
    state.summary = dock.summary;
    state.config = dock.config;
    state.extensions = dock.extensions;
    state.signals = dock.signals;
    if let Some(call) = state
        .transcript
        .items()
        .iter()
        .find(|item| item.status == ItemStatus::Running)
    {
        let id = call.id.clone();
        let slots: Vec<_> = slots
            .into_iter()
            .filter_map(|placed| match placed.slot {
                bingo_sdk::slot::Slot::Row { actions, .. } => Some(bingo_sdk::slot::Slot::Row {
                    item: id.clone(),
                    actions,
                    mark: None,
                    title: None,
                    badge: None,
                    caption: None,
                    body: None,
                }),
                _ => None,
            })
            .collect();
        apply(
            &mut state,
            0,
            extended(
                "bingo.bash",
                "running",
                bingo_sdk::slot::with(json!({}), slots),
            ),
        );
    }
    ui.catalogs.values.insert(
        "tools".into(),
        crate::test_support::ember::story::catalogue(),
    );
    (state, ui)
}

#[test]
fn ember_live_motion_is_only_side_light_spinner_and_comet() {
    let capture = Capture::load("live");
    capture.render(|| {
        let (state, mut ui) = full_fixture(&capture);
        let mut now = later(capture.now(), 700);
        // The budget measures stream motion and the comet after the caret rests.
        now.instant += std::time::Duration::from_secs(10);
        let size = capture.size();
        let a = drawn(size.width, size.height, &solo(&state), &mut ui, now);
        let edge = ui.shown.regions.composer.y;
        let b = drawn(
            size.width,
            size.height,
            &solo(&state),
            &mut ui,
            later(now, 500),
        );
        let cells = moved(a.buffer(), b.buffer());
        assert!(
            cells.contains(&(0, 19))
                && cells.contains(&(3, 25))
                && cells.iter().any(|(_, y)| *y == edge),
            "{cells:?}"
        );
        assert!(
            cells
                .iter()
                .all(|cell| [(0, 19), (3, 25)].contains(cell) || cell.1 == edge),
            "{cells:?}"
        );
    });
}
