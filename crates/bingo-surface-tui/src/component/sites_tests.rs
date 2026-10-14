use crate::test_support::*;
use bingo_sdk::{Event, TurnStatus, render::Component};
use serde_json::json;

fn requests(state: &bingo_sdk::SessionState) -> Vec<Component> {
    let (mut ui, now) = scene();
    draw_tree(80, 24, &solo(state), &mut ui, later(now, 2000));
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
    assert!(requests(&state()).is_empty());
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
    closed.ts = ts() + jiff::SignedDuration::from_secs(2);
    let mut state = folded(vec![frame(1, started("trn_1")), closed]);
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
    assert!(notice < duration);
}
#[test]
fn working_frame_requests_spinner() {
    let mut state = folded(vec![frame(1, started("trn_1"))]);
    matched(&mut state);
    assert!(requests(&state).contains(&Component::Spinner));
}

fn site(component: Component) -> bingo_sdk::SessionState {
    use bingo_sdk::{
        ItemStatus, ToolOutput,
        render::{RenderRequest, SurfaceName, Viewport},
        slot::{Shown, Slot},
    };
    let completed_item = |item| folded(vec![frame(1, Event::ItemCompleted { item })]);
    match component {
        Component::Permission | Component::SignIn => {
            unreachable!("sealed sites have dedicated tests")
        }
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
        Component::ToolUse | Component::ToolResult | Component::ToolGroup => completed_item(tool(
            "itm_one",
            "sample",
            json!({"text":"original"}),
            Some(ToolOutput::text("original")),
            ItemStatus::Completed,
        )),
        Component::ToolProgress => completed_item(running_tool("itm_one", "sample", "original")),
        Component::CommandOutput => {
            completed_item(shell("itm_one", "echo original", "original", Some(0)))
        }
        Component::InfoNotice => completed_item(item(
            "itm_one",
            ItemStatus::Completed,
            bingo_sdk::ItemBody::Notice {
                level: bingo_sdk::Level::Info,
                text: "original".into(),
                code: String::new(),
            },
        )),
        Component::Spinner => folded(vec![frame(1, started("trn_one"))]),
        Component::TurnDuration => {
            let mut closed = frame(2, completed("trn_one", bingo_sdk::TurnStatus::Completed));
            closed.ts = ts() + jiff::SignedDuration::from_secs(2);
            folded(vec![frame(1, started("trn_one")), closed])
        }
        Component::SessionMode => with_permission_mode("plan"),
        Component::PromptHint | Component::AbovePrompt => state(),
        Component::Pane => {
            let mut state = state();
            let slot = Slot::Pane {
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
                    },
                },
            };
            state
                .signals
                .entry("fixture".into())
                .or_default()
                .insert("one".into(), bingo_sdk::slot::with(json!({}), [slot]));
            state
        }
    }
}
fn replaces_drawn_site(component: Component) {
    let mut state = site(component);
    state.config.kernel["render"] = serde_json::to_value(std::collections::BTreeMap::from([(
        component,
        vec![json!({})],
    )]))
    .unwrap();
    let tree = solo(&state);
    let (mut ui, now) = scene();
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
screen_test!(spinner_backend, Spinner);
screen_test!(duration_backend, TurnDuration);
screen_test!(notice_backend, InfoNotice);
screen_test!(mode_backend, SessionMode);
screen_test!(hint_backend, PromptHint);
screen_test!(band_backend, AbovePrompt);
screen_test!(pane_backend, Pane);

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
    let (mut ui, now) = scene();
    let tree = solo(&state);
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
