use crate::test_support::*;
use bingo_sdk::{Event, TurnStatus, render::Component};
use serde_json::json;

fn requests(state: &bingo_sdk::SessionState) -> Vec<Component> {
    let (mut ui, now) = scene();
    draw_tree(80, 24, &solo(state), &mut ui, later(now, 2000));
    ui.shown.pulls.iter().map(|key| key.request.component).collect()
}
fn matched(state: &mut bingo_sdk::SessionState) {
    state.config.kernel["render"] = serde_json::to_value(Component::ALL.into_iter().map(|c| (c, vec![json!({})])).collect::<std::collections::BTreeMap<_, _>>()).unwrap();
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
    let mut closed = frame(2, completed("trn_1", TurnStatus::Failed { error: bingo_sdk::KernelError::new(bingo_sdk::ErrorCode::Internal, "completion notice") }));
    closed.ts = ts() + jiff::SignedDuration::from_secs(2);
    let mut state = folded(vec![frame(1, started("trn_1")), closed]);
    matched(&mut state);
    let asked = requests(&state);
    let notice = asked.iter().position(|c| *c == Component::InfoNotice).expect("notice");
    let duration = asked.iter().position(|c| *c == Component::TurnDuration).expect("duration");
    assert!(notice < duration);
}
#[test]
fn working_frame_requests_spinner() {
    let mut state = folded(vec![frame(1, started("trn_1"))]);
    matched(&mut state);
    assert!(requests(&state).contains(&Component::Spinner));
}
