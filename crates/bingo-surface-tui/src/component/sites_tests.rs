use crate::test_support::*;
use bingo_sdk::{Event, TurnStatus, render::Component};
use serde_json::json;

fn requests(state: &bingo_sdk::SessionState) -> Vec<Component> {
    requests_in(&solo(state))
}
fn requests_in(tree: &crate::tree::Tree) -> Vec<Component> {
    let (mut ui, now) = reading(tree);
    draw_tree(80, 24, tree, &mut ui, later(now, 2000));
    ui.shown
        .pulls
        .iter()
        .map(|key| key.request.component)
        .collect()
}
fn matched(state: &mut bingo_sdk::SessionState) {
    state.config.kernel["render"] = serde_json::to_value(
        Component::ALL
            .into_iter()
            .map(|c| (c, vec![json!({})]))
            .collect::<std::collections::BTreeMap<_, _>>(),
    )
    .unwrap();
}
#[test]
fn unmatched_frame_has_no_component_requests() {
    for component in Component::ALL {
        let state = site(component);
        let asked = requests_in(&seen(component, &state));
        assert!(asked.is_empty(), "{component:?}");
    }
}
#[test]
fn permission_never_requests_question_even_with_unrestricted_hook() {
    let interaction = permission(None, None);
    let mut state = folded(vec![frame(1, Event::InteractionOpened { interaction })]);
    matched(&mut state);
    assert!(!requests(&state).contains(&Component::Question));
}
#[test]
fn question_requests_its_typed_component() {
    let interaction = question(false, true);
    let mut state = folded(vec![frame(1, Event::InteractionOpened { interaction })]);
    matched(&mut state);
    assert!(requests(&state).contains(&Component::Question));
}
#[test]
fn turn_complete_notice_and_duration_are_separate_ordered_requests() {
    let mut closed = frame(
        2,
        completed(
            "trn_1",
            TurnStatus::Failed {
                error: bingo_sdk::KernelError::new(
                    bingo_sdk::ErrorCode::Internal,
                    "completion notice",
                ),
            },
        ),
    );
    closed.seq = bingo_sdk::Seq(3);
    closed.ts = ts() + jiff::SignedDuration::from_secs(2);
    let mut prompt = user("prompt", "go");
    prompt.completed_at = Some(closed.ts);
    let mut state = folded(vec![
        frame(1, started("trn_1")),
        frame(2, Event::ItemCompleted { item: prompt }),
        closed,
    ]);
    matched(&mut state);
    let asked = requests(&state);
    let notice = asked
        .iter()
        .position(|c| *c == Component::InfoNotice)
        .expect("notice");
    let duration = asked
        .iter()
        .position(|c| *c == Component::TurnDuration)
        .expect("duration");
    assert!(duration < notice);
}

/// Where each site is drawn in a session a model answers: every item in a
/// turn.
fn site(component: Component) -> bingo_sdk::SessionState {
    use bingo_sdk::ItemStatus;
    let completed_item = |item| folded(vec![frame(1, Event::ItemCompleted { item })]);
    match component {
        Component::KeyBar
        | Component::Composer
        | Component::ModelPicker
        | Component::Browse
        | Component::Palette
        | Component::Plugins
        | Component::Sessions => state(),
        Component::Permission | Component::SignIn => sealed_site(component),
        Component::Question => folded(vec![frame(
            1,
            Event::InteractionOpened {
                interaction: question(false, true),
            },
        )]),
        Component::UserMessage => completed_item(user("itm_one", "original")),
        Component::AssistantMessage => {
            completed_item(assistant("itm_one", "original", ItemStatus::Completed))
        }
        Component::ToolUse
        | Component::ToolGroup
        | Component::ToolResult
        | Component::ToolProgress
        | Component::Ledger
        | Component::TurnFold => call_site(component),
        Component::CommandOutput => completed_item(item(
            "itm_one",
            ItemStatus::Completed,
            bingo_sdk::ItemBody::Action {
                name: "sample".into(),
                args: json!({}),
                result: Some(json!("original")),
            },
        )),
        Component::InfoNotice => completed_item(item(
            "itm_one",
            ItemStatus::Completed,
            bingo_sdk::ItemBody::Notice {
                level: bingo_sdk::Level::Info,
                text: "original".into(),
                code: String::new(),
            },
        )),
        Component::Spinner | Component::TurnProgress => folded(vec![frame(1, started("trn_one"))]),
        Component::TurnDuration => {
            let mut closed = frame(2, completed("trn_one", bingo_sdk::TurnStatus::Completed));
            closed.seq = bingo_sdk::Seq(3);
            closed.ts = ts() + jiff::SignedDuration::from_secs(2);
            {
                let mut prompt = user("prompt", "go");
                prompt.turn = Some(bingo_sdk::TurnId::from_raw("trn_one"));
                prompt.completed_at = Some(closed.ts);
                folded(vec![
                    frame(1, started("trn_one")),
                    frame(2, Event::ItemCompleted { item: prompt }),
                    closed,
                ])
            }
        }
        Component::SessionMode => with_permission_mode("plan"),
        Component::PromptHint
        | Component::AbovePrompt
        | Component::SessionStatus
        | Component::Welcome => state(),
        Component::Toast | Component::Pane | Component::Status => placed_site(component),
        Component::Reasoning
        | Component::Compaction
        | Component::Rewind
        | Component::Interruption
        | Component::QuestionAnswer
        | Component::PermissionReceipt
        | Component::Asset
        | Component::ShellRun
        | Component::Action => fact_site(component),
    }
}
/// Where a call's sites are drawn. A step asks `Ledger` for its row and, its
/// turn opened, `ToolResult` for what it brought back; a call that spawned an
/// agent is its own row and asks `ToolUse` and `ToolGroup`; a running call a
/// producer drew a row for asks `ToolProgress`.
fn call_site(component: Component) -> bingo_sdk::SessionState {
    let completed = |id: &str| {
        let item = tool(
            id,
            "sample",
            json!({"text":"original"}),
            Some(bingo_sdk::ToolOutput::text("original")),
            bingo_sdk::ItemStatus::Completed,
        );
        folded(vec![frame(1, Event::ItemCompleted { item })])
    };
    let running = || {
        folded(vec![
            frame(1, started("trn_1")),
            started_tool(2, running_tool("itm_one", "sample", "original")),
        ])
    };
    match component {
        Component::ToolUse | Component::ToolGroup => completed(SPAWNING),
        Component::ToolProgress => crate::transcript::slots::fixtures::with_row(
            running(),
            bingo_sdk::slot::Slot::Row {
                item: bingo_sdk::ItemId::from_raw("itm_one"),
                title: Some("Review".into()),
                mark: None,
                badge: None,
                caption: None,
                body: None,
                actions: vec![],
            },
        ),
        Component::Ledger => running(),
        _ => completed("itm_one"),
    }
}

/// The call [`child_summary`]'s agent was spawned by.
const SPAWNING: &str = "itm_1";

/// The tree a site's session is seen in: for a call that spawned an agent,
/// with that agent under it.
fn seen(component: Component, state: &bingo_sdk::SessionState) -> crate::tree::Tree {
    let mut tree = solo(state);
    if matches!(component, Component::ToolUse | Component::ToolGroup) {
        tree.fold.apply(&child_frame(1, announced("helper")));
    }
    tree
}

/// A fresh screen with every finished turn opened, as a person reading what
/// the turns did has it.
fn reading(tree: &crate::tree::Tree) -> (crate::ui::Ui, crate::clock::Now) {
    let (mut ui, now) = scene();
    open_the_turns(&mut ui, tree.viewed());
    (ui, now)
}

fn replaces_drawn_site(component: Component) {
    let mut state = site(component);
    state.config.kernel["render"] = serde_json::to_value(std::collections::BTreeMap::from([(
        component,
        vec![json!({})],
    )]))
    .unwrap();
    let tree = seen(component, &state);
    let (mut ui, now) = reading(&tree);
    let now = later(now, 2000);
    draw_tree(80, 30, &tree, &mut ui, now);
    let key = ui
        .shown
        .pulls
        .iter()
        .find(|key| key.request.component == component)
        .expect("drawn component asks")
        .clone();
    bingo_sdk::component::Props::parse(component, key.request.props.clone())
        .expect("typed site projection");
    assert!(ui.placed.pulled.request(&key));
    ui.placed
        .pulled
        .answer(key, bingo_sdk::View::text("component replacement"));
    let screen = draw_tree(80, 30, &tree, &mut ui, now);
    assert!(
        screen.contains("component replacement"),
        "{component:?}: {screen}"
    );
}
macro_rules! screen_test {
    ($name:ident, $component:ident) => {
        #[test]
        fn $name() {
            replaces_drawn_site(Component::$component);
        }
    };
}
screen_test!(question_backend, Question);
screen_test!(user_backend, UserMessage);
screen_test!(assistant_backend, AssistantMessage);
screen_test!(tool_use_backend, ToolUse);
screen_test!(tool_result_backend, ToolResult);
screen_test!(tool_group_backend, ToolGroup);
screen_test!(tool_progress_backend, ToolProgress);
screen_test!(command_output_backend, CommandOutput);

screen_test!(duration_backend, TurnDuration);
screen_test!(notice_backend, InfoNotice);
screen_test!(mode_backend, SessionMode);
screen_test!(keybar_backend, KeyBar);
screen_test!(band_backend, AbovePrompt);
screen_test!(pane_backend, Pane);
screen_test!(status_backend, Status);

#[tokio::test]
async fn counting_host_receives_no_unmatched_render_requests() {
    let host = bingo_sdk::testing::TestHost::default();
    for expected in [0, 1] {
        let mut state = site(Component::AssistantMessage);
        if expected == 1 {
            state.config.kernel["render"] = json!({"AssistantMessage":[{}]});
        }
        let (mut ui, now) = scene();
        draw_tree(80, 24, &solo(&state), &mut ui, now);
        for key in &ui.shown.pulls {
            let _ = host
                .handle()
                .render(&key.session, key.request.clone())
                .await;
        }
        assert_eq!(
            host.calls()
                .iter()
                .filter(|call| matches!(call, bingo_sdk::testing::HostCall::Render { .. }))
                .count(),
            expected
        );
    }
}

#[test]
fn a_replacement_group_hides_its_child_requests() {
    let mut state = site(Component::ToolGroup);
    matched(&mut state);
    let tree = seen(Component::ToolGroup, &state);
    let (mut ui, now) = reading(&tree);
    draw_tree(80, 30, &tree, &mut ui, now);
    let key = ui
        .shown
        .pulls
        .iter()
        .find(|key| key.request.component == Component::ToolGroup)
        .unwrap()
        .clone();
    ui.placed.pulled.request(&key);
    ui.placed
        .pulled
        .answer(key, bingo_sdk::View::text("group only"));
    draw_tree(80, 30, &tree, &mut ui, now);
    assert!(!ui.shown.pulls.iter().any(|key| matches!(
        key.request.component,
        Component::ToolUse | Component::ToolResult
    )));
}
#[test]
fn unmatched_pane_keeps_its_chrome_without_requesting_a_hook() {
    assert!(requests(&site(Component::Pane)).is_empty());
}
#[test]
fn removing_a_match_discards_the_cached_replacement() {
    let mut state = site(Component::AssistantMessage);
    matched(&mut state);
    let (mut ui, now) = scene();
    draw_tree(80, 24, &solo(&state), &mut ui, now);
    let key = ui
        .shown
        .pulls
        .iter()
        .find(|key| key.request.component == Component::AssistantMessage)
        .unwrap()
        .clone();
    ui.placed.pulled.request(&key);
    ui.placed
        .pulled
        .answer(key, bingo_sdk::View::text("replacement"));
    assert!(draw_tree(80, 24, &solo(&state), &mut ui, now).contains("replacement"));
    state.config.kernel["render"] = json!({});
    assert!(!draw_tree(80, 24, &solo(&state), &mut ui, now).contains("replacement"));
    assert!(ui.shown.pulls.is_empty());
}

#[test]
fn revalidating_above_prompt_keeps_previous_art() {
    let mut state = site(Component::AbovePrompt);
    state.config.kernel["render"] = json!({"AbovePrompt":[{}]});
    let (mut ui, now) = scene();
    draw_sized(80, 30, &state, &mut ui, now);
    let first = ui
        .shown
        .pulls
        .iter()
        .find(|key| key.request.component == Component::AbovePrompt)
        .unwrap()
        .clone();
    ui.placed.pulled.request(&first);
    ui.placed
        .pulled
        .answer(first.clone(), bingo_sdk::View::text("BAND ART"));
    assert!(draw_sized(80, 30, &state, &mut ui, now).contains("BAND ART"));
    state
        .invalidated
        .insert(first.request.request_id.clone(), 1);
    draw_sized(40, 24, &state, &mut ui, now);
    let next = ui
        .shown
        .pulls
        .iter()
        .find(|key| key.request.component == Component::AbovePrompt)
        .unwrap()
        .clone();
    ui.placed.pulled.retain(std::slice::from_ref(&next));
    ui.placed.pulled.request(&next);
    let screen = draw_sized(40, 24, &state, &mut ui, now);
    assert!(screen.contains("BAND ART"), "{screen}");
    ui.placed
        .pulled
        .answer(next, bingo_sdk::View::text("NEW BAND"));
    assert!(draw_sized(40, 24, &state, &mut ui, now).contains("NEW BAND"));
}

fn fact_site(component: Component) -> bingo_sdk::SessionState {
    let fixtures: Vec<serde_json::Value> =
        serde_json::from_str(include_str!("../../../bingo-sdk/src/component/props.json")).unwrap();
    let props: bingo_sdk::component::Props = serde_json::from_value(
        fixtures
            .into_iter()
            .find(|f| f["component"] == json!(component))
            .unwrap(),
    )
    .unwrap();
    // In a turn, opened: a thought is read there, a stop drawn there.
    let mut record = item(
        "itm_one",
        bingo_sdk::ItemStatus::Completed,
        bingo_sdk::ItemBody::Reasoning {
            text: String::new(),
            provider_metadata: Default::default(),
        },
    );
    super::facts::change(&mut record, &props);
    folded(vec![frame(
        1,
        Event::ItemCompleted {
            item: loose(record),
        },
    )])
}

#[test]
fn every_known_item_body_has_a_site() {
    use bingo_sdk::ItemBody;
    for component in Component::ALL {
        let state = site(component);
        for item in state.transcript.items() {
            let known = match item.body {
                ItemBody::User { .. }
                | ItemBody::Assistant { .. }
                | ItemBody::Reasoning { .. }
                | ItemBody::ToolCall { .. }
                | ItemBody::Shell { .. }
                | ItemBody::Action { .. }
                | ItemBody::Compaction { .. }
                | ItemBody::Rewind { .. }
                | ItemBody::Interruption { .. }
                | ItemBody::Notice { .. }
                | ItemBody::QuestionAnswer { .. }
                | ItemBody::PermissionReceipt { .. }
                | ItemBody::Asset { .. } => true,
                ItemBody::Unknown { .. } => false,
            };
            assert_eq!(super::items::props(item, false).is_some(), known);
        }
    }
}

screen_test!(reasoning_backend, Reasoning);
screen_test!(compaction_backend, Compaction);
screen_test!(rewind_backend, Rewind);
screen_test!(interruption_backend, Interruption);
screen_test!(questionanswer_backend, QuestionAnswer);
screen_test!(permissionreceipt_backend, PermissionReceipt);
screen_test!(asset_backend, Asset);
screen_test!(shellrun_backend, ShellRun);
screen_test!(action_backend, Action);

screen_test!(sessionstatus_backend, SessionStatus);
screen_test!(toast_backend, Toast);
screen_test!(welcome_backend, Welcome);
screen_test!(ledger_backend, Ledger);

#[test]
fn every_builtin_site_keeps_pixels_when_answered_with_engine() {
    for component in Component::ALL {
        // Pulled sites wait for their first answer; they have no surface body.
        if matches!(
            component,
            Component::Pane
                | Component::Status
                | Component::ModelPicker
                | Component::PromptHint
                | Component::Composer
        ) {
            continue;
        }
        let mut state = site(component);
        matched(&mut state);
        let tree = seen(component, &state);
        let (mut ui, now) = reading(&tree);
        let now = later(now, 2000);
        if component == Component::Browse {
            crate::browse::open(&mut ui, &tree, None, false, now);
        }
        if component == Component::Palette {
            crate::palette::open(&mut ui, bingo_sdk::component::PaletteOpen::Commands, now);
        } else if component == Component::Plugins {
            crate::plugins::open(&mut ui, bingo_sdk::component::PluginsOpen::Plugins, now);
        }
        if component == Component::Sessions {
            crate::sessions::open(&mut ui, now);
        }
        let original = drawn(80, 30, &tree, &mut ui, now);
        let key = ui
            .shown
            .pulls
            .iter()
            .find(|key| key.request.component == component)
            .unwrap()
            .clone();
        ui.placed.pulled.request(&key);
        let answer = key.request.engine();
        ui.placed.pulled.answer(key, answer);
        let wrapped = drawn(80, 30, &tree, &mut ui, now);
        assert_eq!(original.buffer(), wrapped.buffer(), "{component:?}");
    }
}

#[test]
fn a_rewritten_fact_reaches_the_own_drawing() {
    let mut state = site(Component::Compaction);
    matched(&mut state);
    let (mut ui, now) = scene();
    let tree = solo(&state);
    draw_tree(80, 30, &tree, &mut ui, now);
    let key = ui
        .shown
        .pulls
        .iter()
        .find(|key| key.request.component == Component::Compaction)
        .unwrap()
        .clone();
    let mut props = key.request.props.clone();
    props["before"] = json!(1234);
    props["after"] = json!(432);
    ui.placed.pulled.request(&key);
    ui.placed.pulled.answer(
        key,
        bingo_sdk::View::Engine {
            component: Component::Compaction,
            props,
        },
    );
    let screen = draw_tree(80, 30, &tree, &mut ui, now);
    assert!(screen.contains("1234 → 432"), "{screen}");
}

fn placed_site(component: Component) -> bingo_sdk::SessionState {
    use bingo_sdk::{
        render::{RenderRequest, SurfaceName, Viewport},
        slot::{Shown, Slot},
    };
    match component {
        Component::Toast => {
            let mut state = state();
            state.signals.entry("fixture".into()).or_default().insert(
                "toast".into(),
                bingo_sdk::slot::with(
                    json!({}),
                    [Slot::Toast {
                        text: "original".into(),
                        until: ts() + std::time::Duration::from_secs(10),
                        alert: false,
                    }],
                )
                .into(),
            );
            state
        }
        Component::Pane | Component::Status => {
            let mut state = state();
            let slot = Slot::Pane {
                hints: None,
                id: "one".into(),
                title: "original".into(),
                focus: false,
                close_on_escape: true,
                hold_toasts: false,
                asked: true,
                place: vec![bingo_sdk::slot::Place::Above],
                size: bingo_sdk::slot::PaneSize::from_rows(5),
                view: Shown::Pull {
                    request: RenderRequest {
                        component,
                        request_id: "one".into(),
                        surface: SurfaceName::Terminal,
                        props: json!({}),
                        viewport: Viewport {
                            place: None,
                            columns: 0,
                            rows: 0,
                        },
                    }
                    .into(),
                },
            };
            state.signals.entry("fixture".into()).or_default().insert(
                "one".into(),
                bingo_sdk::slot::with(json!({}), [slot]).into(),
            );
            state
        }
        _ => unreachable!(),
    }
}

#[test]
fn session_status_replacement_owns_the_child_identity() {
    let mut state = site(Component::SessionStatus);
    state.config.kernel["render"] = json!({"SessionStatus":[{}]});
    let mut tree = folded_tree(vec![
        child_frame(1, announced("reviewer")),
        child_frame(
            2,
            Event::ConfigChanged {
                config: state.config,
            },
        ),
    ]);
    tree.show(&child_id());
    let (mut ui, now) = scene();
    let original = draw_tree(80, 30, &tree, &mut ui, now);
    assert!(original.contains("Message reviewer"));
    let key = ui
        .shown
        .pulls
        .iter()
        .find(|key| key.request.component == Component::SessionStatus)
        .unwrap()
        .clone();
    assert_eq!(key.request.props["title"], json!("reviewer"));
    assert_eq!(key.request.props["cwd"], json!(tree.viewed().summary.cwd));
    ui.placed.pulled.request(&key);
    ui.placed
        .pulled
        .answer(key, bingo_sdk::View::text("replacement status"));
    let screen = draw_tree(80, 30, &tree, &mut ui, now);
    assert!(screen.contains("replacement status"));
    assert!(screen.contains("Message reviewer"));
    assert!(!screen.contains("fake/fake-1"));
}

fn sealed_site(component: Component) -> bingo_sdk::SessionState {
    let interaction = if component == Component::Permission {
        interaction(
            bingo_sdk::InteractionKind::Permission {
                tool: "sample".into(),
                summary: "sample operation".into(),
                preview: None,
                session_scope: None,
                caution: None,
                input: Default::default(),
            },
            vec![
                bingo_sdk::AnswerSpec::AllowOnce,
                bingo_sdk::AnswerSpec::Deny,
            ],
        )
    } else {
        login(bingo_sdk::LoginFlow::Device {
            url: "https://example.test/device".into(),
            code: "ABCD".into(),
        })
    };
    folded(vec![
        started_tool(1, running_tool("itm_2", "sample", "original")),
        frame(2, opened(interaction)),
    ])
}

#[test]
fn flattened_toast_has_no_motion_deadline() {
    let mut state = site(Component::Toast);
    matched(&mut state);
    let (mut ui, now) = scene();
    ui.sync_toasts(&state, now);
    let mut frame = crate::layout::Layout::default();
    crate::toast::components(&ui, &state, now, 80, &mut frame.pulls).unwrap();
    let key = frame
        .pulls
        .iter()
        .find(|key| key.request.component == Component::Toast)
        .unwrap()
        .clone();
    ui.placed.pulled.request(&key);
    ui.placed.pulled.answer(
        key,
        serde_json::from_value(json!({
            "kind":"Text", "motion":{"kind":"breathe","with":"glow","periodMs":1800,"depth":1.0},
            "children":["still words"]
        }))
        .unwrap(),
    );
    crate::theme::with(crate::painted::daylight(), || {
        let toast = crate::toast::components(&ui, &state, now, 80, &mut frame.pulls).unwrap();
        assert_eq!(toast.plain().unwrap().text, "still words");
        assert_eq!(frame.deadline(), None);
    });
}
