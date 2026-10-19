//! Kernel answers used by acting scenes; proofs exercise these against the real Host.
use crate::test_support::{completed, item};
use bingo_sdk::*;
use jiff::Timestamp;

pub fn queued(state: &SessionState, intent: IntentId, input: &Input, revision: u64) -> Vec<Event> {
    let Input::Text { text, origin, .. } = input else {
        panic!("text input");
    };
    let mut entries = state.queue.clone();
    let position = entries.len() as u32 + 1;
    entries.push(QueueEntry {
        intent: intent.clone(),
        position,
        preview: text.clone(),
        steerable: !input.is_held(),
        origin: origin.clone(),
    });
    vec![
        Event::QueueChanged { revision, entries },
        Event::IntentAck {
            intent,
            outcome: IntentOutcome::Queued { position },
        },
    ]
}

pub fn removed(state: &SessionState, intent: &IntentId, revision: u64) -> Event {
    let entries = state
        .queue
        .iter()
        .filter(|q| &q.intent != intent)
        .enumerate()
        .map(|(n, q)| QueueEntry {
            position: n as u32 + 1,
            ..q.clone()
        })
        .collect();
    Event::QueueChanged { revision, entries }
}

fn user(id: &str, turn: &str, intent: Option<IntentId>, input: &Input, at: Timestamp) -> Item {
    let Input::Text {
        text,
        images,
        origin,
        headline,
        ..
    } = input
    else {
        panic!("text input");
    };
    let mut parts = vec![];
    if !text.is_empty() {
        parts.push(ContentPart::text(text));
    }
    parts.extend(images.iter().cloned().map(ContentPart::Image));
    let mut user = item(
        id,
        ItemStatus::Completed,
        ItemBody::User {
            parts,
            origin: origin.clone(),
        },
    );
    user.turn = Some(TurnId::from_raw(turn));
    user.intent = intent;
    user.started_at = at;
    user.completed_at = Some(at);
    if let Some(headline) = headline {
        user.set_headline(headline.clone());
    }
    user
}

pub fn sent(turn: &str, intent: IntentId, input: &Input, at: Timestamp) -> Vec<Event> {
    let item = user(
        &format!("{turn}-prompt"),
        turn,
        Some(intent.clone()),
        input,
        at,
    );
    vec![
        Event::ItemCompleted { item: item.clone() },
        Event::TurnStarted {
            turn: TurnId::from_raw(turn),
            inputs: vec![item.id],
            origin: TurnOrigin::Submit,
        },
        Event::IntentAck {
            intent,
            outcome: IntentOutcome::TurnStarted {
                turn: TurnId::from_raw(turn),
            },
        },
    ]
}

pub fn steered(
    state: &SessionState,
    intent: &IntentId,
    input: &Input,
    id: &str,
    at: Timestamp,
    revision: u64,
) -> Vec<Event> {
    let turn = state.turn.as_ref().unwrap().id.to_string();
    vec![
        removed(state, intent, revision),
        Event::ItemCompleted {
            item: user(id, &turn, None, input, at),
        },
    ]
}

pub fn withdrawn(input: &Input) -> Result<Input, KernelError> {
    Ok(input.clone())
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum StopPhase {
    Streaming,
    Tools,
}

pub fn stopped(state: &SessionState, at: Timestamp, phase: StopPhase) -> Vec<Event> {
    let turn = &state.turn.as_ref().unwrap().id;
    let mut events = Vec::new();
    for mut item in state
        .transcript
        .items()
        .iter()
        .filter(|i| i.turn.as_ref() == Some(turn) && !i.is_terminal())
        .map(|item| item.as_ref().clone())
    {
        if phase == StopPhase::Streaming
            && matches!(item.body, ItemBody::ToolCall { .. })
            && item.status == ItemStatus::Pending
        {
            continue;
        }
        item.status = ItemStatus::Interrupted;
        item.completed_at = Some(at);
        if let ItemBody::ToolCall {
            output,
            progress,
            duration_ms,
            began_at,
            ..
        } = &mut item.body
        {
            *output = Some(Box::new(bingo_sdk::ToolOutput::error(
                bingo_core::executor::INTERRUPTED_MARKER,
            )));
            *progress = None;
            *duration_ms = Some(began_at.map_or(0, |began| {
                at.as_millisecond()
                    .saturating_sub(began.as_millisecond())
                    .max(0) as u64
            }));
        }
        events.push(Event::ItemCompleted { item });
    }
    let mut marker = item(
        "interruption",
        ItemStatus::Completed,
        ItemBody::Interruption {
            asked_at: None,
            marker: bingo_core::turn::INTERRUPTED_MARKER.into(),
        },
    );
    marker.turn = Some(turn.clone());
    marker.started_at = at;
    marker.completed_at = Some(at);
    events.push(Event::ItemCompleted { item: marker });
    events.push(completed(
        &turn.to_string(),
        TurnStatus::Interrupted {
            reason: InterruptReason::UserCancel,
        },
    ));
    events
}

pub fn shell(
    command: &str,
    output: &str,
    cwd: &str,
    at: Timestamp,
    id: &str,
    intent: IntentId,
) -> Event {
    let mut item = item(
        id,
        ItemStatus::Completed,
        ItemBody::Shell {
            command: command.trim_start_matches('!').into(),
            output: output.into(),
            exit: Some(0),
            cwd: cwd.into(),
        },
    );
    item.turn = None;
    item.intent = Some(intent);
    item.started_at = at;
    item.completed_at = Some(at);
    Event::ItemCompleted { item }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::{folded, frame, started, tool, ts};

    #[test]
    fn a_queued_chain_call_gets_the_tool_marker_and_zero_duration() {
        let call = tool(
            "queued-call",
            "Bash",
            serde_json::json!({}),
            None,
            ItemStatus::Pending,
        );
        let state = folded(vec![
            frame(1, started("trn_1")),
            frame(2, Event::ItemStarted { item: call }),
        ]);
        let events = stopped(
            &state,
            ts() + std::time::Duration::from_secs(1),
            StopPhase::Tools,
        );
        let Event::ItemCompleted { item } = &events[0] else {
            panic!("the queued call completes");
        };
        assert_eq!(item.status, ItemStatus::Interrupted);
        let ItemBody::ToolCall {
            output,
            duration_ms,
            progress,
            ..
        } = &item.body
        else {
            panic!("the call stays a call");
        };
        assert_eq!(
            output.as_deref().unwrap(),
            &ToolOutput::error(bingo_core::executor::INTERRUPTED_MARKER)
        );
        assert_eq!(*duration_ms, Some(0));
        assert!(progress.is_none());
    }
}
