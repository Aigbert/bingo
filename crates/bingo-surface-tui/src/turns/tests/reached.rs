use super::*;
use crate::test_support::*;
use bingo_sdk::Event;

#[test]
fn stamps_and_status_decide_whether_a_step_exists() {
    let state = folded(running_turn());
    for (status, asked, began, expected) in [
        (ItemStatus::Pending, false, false, None),
        (ItemStatus::Pending, true, false, Some(StepState::Asked)),
        (ItemStatus::Running, true, false, Some(StepState::Asked)),
        (ItemStatus::Running, false, true, Some(StepState::Running)),
        (ItemStatus::Interrupted, false, false, None),
        (
            ItemStatus::Interrupted,
            true,
            false,
            Some(StepState::Stopped),
        ),
        (
            ItemStatus::Interrupted,
            false,
            true,
            Some(StepState::Stopped),
        ),
        (ItemStatus::Failed, false, false, Some(StepState::Failed)),
        (ItemStatus::Completed, false, false, Some(StepState::Done)),
    ] {
        let mut item = tool("call", "Edit", serde_json::json!({}), None, status);
        if let ItemBody::ToolCall {
            asked_at, began_at, ..
        } = &mut item.body
        {
            *asked_at = asked.then(ts);
            *began_at = began.then(ts);
        }
        assert_eq!(
            step_state(&item, &state),
            expected,
            "{status:?}/{asked}/{began}"
        );
    }
}

fn lines(state: &SessionState) -> (String, Vec<String>) {
    let folds = Default::default();
    let pictures = Default::default();
    let linked = Default::default();
    let rows = crate::transcript::Rows::of(
        state,
        80,
        &folds,
        &pictures,
        &linked,
        still(later(scene().1, 9000)),
    );
    let roles = Roles::of(&Agents::new(), &rows);
    let mut text = String::new();
    let mut requested = Vec::new();
    for item in state.transcript.items() {
        let role = roles.of_item(&item.id);
        requested.extend(requests(item, role, &rows).into_iter().map(|(id, _)| id));
        if let Role::Step { step, .. } = role {
            text.push_str(&crate::transcript::asked(step, 80).to_string());
            text.push('\n');
        }
    }
    assert!(
        crate::component::live::props(&rows).is_none(),
        "live row flashed: {text}"
    );
    (text, requested)
}

#[test]
fn a_held_step_stays_until_it_runs_or_stops_and_never_reveals_the_next_call() {
    let read = tool(
        "read",
        "Read",
        serde_json::json!({"file_path":"first.txt"}),
        None,
        ItemStatus::Completed,
    );
    let mut edit = tool(
        "edit",
        "Edit",
        serde_json::json!({"file_path":"note.txt"}),
        None,
        ItemStatus::Pending,
    );
    let bash = tool(
        "bash",
        "Bash",
        serde_json::json!({"command":"echo never"}),
        None,
        ItemStatus::Pending,
    );
    let mut state = folded(vec![
        frame(1, started("trn_1")),
        frame(2, Event::ItemCompleted { item: read }),
        frame(3, Event::ItemStarted { item: edit.clone() }),
        frame(4, Event::ItemStarted { item: bash }),
    ]);
    if let ItemBody::ToolCall { asked_at, .. } = &mut edit.body {
        *asked_at = Some(ts());
    }
    state.apply(&frame(5, Event::ItemUpdated { item: edit.clone() }));
    let waiting = state.clone();
    for (index, event) in [
        None,
        Some(Event::InteractionOpened {
            interaction: permission_for(&edit),
        }),
        Some(Event::InteractionResolved {
            id: bingo_sdk::InteractionId::from_raw("ask"),
            answer: bingo_sdk::Answer::AllowOnce,
            by: bingo_sdk::ResolvedBy::Client {
                name: "test".into(),
                surface: "test".into(),
            },
        }),
    ]
    .into_iter()
    .enumerate()
    {
        if let Some(event) = event {
            state.apply(&frame(6 + index as u64, event));
        }
        let (text, requested) = lines(&state);
        assert!(
            text.contains("first.txt") && text.contains("note.txt") && text.contains("needs you"),
            "{text}"
        );
        assert!(!text.contains("echo never"));
        assert!(!requested.iter().any(|id| id.contains("bash")));
    }
    edit.status = ItemStatus::Running;
    if let ItemBody::ToolCall { began_at, .. } = &mut edit.body {
        *began_at = Some(ts());
    }
    state.apply(&frame(9, Event::ItemUpdated { item: edit.clone() }));
    assert!(!lines(&state).0.contains("needs you"));
    for mut state in [waiting, state] {
        for mut item in state
            .transcript
            .items()
            .iter()
            .map(|i| i.as_ref().clone())
            .collect::<Vec<_>>()
        {
            if item.status.is_terminal() {
                continue;
            }
            item.status = ItemStatus::Interrupted;
            item.completed_at = Some(ts());
            state.apply(&frame(10, Event::ItemCompleted { item }));
        }
        state.turn = None;
        let (text, _) = lines(&state);
        assert!(
            text.lines()
                .any(|line| line.contains("note.txt") && line.contains("stopped")),
            "{text}"
        );
        assert!(!text.contains("echo never"));
    }
}

fn permission_for(item: &Item) -> bingo_sdk::Interaction {
    serde_json::from_value(serde_json::json!({"id":"ask","session":"s","turn":"trn_1","item":item.id,
        "openedAt":ts(),"kind":{"kind":"permission","tool":"Edit","input":{},"summary":"Allow?"},"answers":["allowOnce"]})).unwrap()
}

#[test]
fn unreached_calls_are_hidden_even_when_quiet_and_old_stops_stay_hidden() {
    for status in [ItemStatus::Pending, ItemStatus::Interrupted] {
        let mut call = tool("hidden", "TaskList", serde_json::json!({}), None, status);
        if let ItemBody::ToolCall { began_at, .. } = &mut call.body {
            *began_at = None;
        }
        let state = folded(vec![frame(1, Event::ItemCompleted { item: call.clone() })]);
        let folds = Default::default();
        let pictures = Default::default();
        let linked = Default::default();
        let rows = crate::transcript::Rows::of(&state, 80, &folds, &pictures, &linked, scene().1);
        let roles = Roles::of(&Agents::new(), &rows);
        assert_eq!(roles.of_item(&call.id), &Role::Hidden);
        assert!(requests(&call, roles.of_item(&call.id), &rows).is_empty());
    }
}

#[test]
fn an_agent_that_began_is_stopped_and_an_old_open_interaction_still_asks() {
    let mut call = tool(
        "agent",
        "agent:edit",
        serde_json::json!({}),
        None,
        ItemStatus::Interrupted,
    );
    call.mark_external();
    let mut state = folded(vec![]);
    assert_eq!(step_state(&call, &state), Some(StepState::Stopped));
    call.status = ItemStatus::Pending;
    call.completed_at = None;
    if let ItemBody::ToolCall { began_at, .. } = &mut call.body {
        *began_at = None;
    }
    state.interactions.push(permission_for(&call));
    assert_eq!(step_state(&call, &state), Some(StepState::Asked));
}
