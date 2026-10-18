use super::*;
use crate::turn::{HookSet, TurnOutcome};
use tokio::sync::oneshot;

struct EndItems(mpsc::UnboundedSender<Vec<Item>>);
#[async_trait::async_trait]
impl Hook for EndItems {
    fn id(&self) -> &str {
        "end-items"
    }
    fn matcher(&self) -> HookMatcher {
        HookMatcher {
            points: vec![HookPoint::Turn],
            tool: None,
        }
    }
    async fn on_turn(&self, phase: Phase, _: &TurnId, items: &[Item], _: &HookContext) {
        if phase == Phase::End {
            self.0.send(items.to_vec()).unwrap();
        }
    }
}

async fn observed_actor(permission: bool) -> (Actor, mpsc::UnboundedReceiver<Vec<Item>>) {
    let (mut actor, _) = storage::failing_actor().await;
    let (send, receive) = mpsc::unbounded_channel();
    let script = if permission {
        Script::Events(tool_call("Echo", json!({"v": 1})))
    } else {
        Script::Hang(vec![ModelEvent::TextStart { id: "live".into() }])
    };
    let mut cfg = config(
        ScriptedProvider::new(vec![script]),
        vec![Arc::new(EchoTool { read_only: false })],
        Arc::new(NoHost),
    );
    cfg.hooks = HookSet::fixed(vec![Arc::new(EndItems(send))]);
    actor.config = Arc::new(cfg);
    actor
        .start_turn(
            vec![(IntentId::mint(), Input::text("go", Origin::surface("test")))],
            TurnOrigin::Submit,
            TurnKind::Respond,
        )
        .await
        .unwrap();
    loop {
        let message = actor.rx.recv().await.unwrap();
        let started = matches!(&message, Msg::Emit { event, .. } if matches!(event.as_ref(), Event::ItemStarted { .. }));
        actor.handle(message).await;
        if (permission && !actor.state.interactions.is_empty()) || (!permission && started) {
            break;
        }
    }
    (actor, receive)
}

async fn returned(actor: &mut Actor) -> TurnOutcome {
    loop {
        match actor.rx.recv().await.unwrap() {
            Msg::TurnFinished {
                outcome: Ended::Returned(out),
                ..
            } => return out,
            message => {
                actor.handle(message).await;
            }
        }
    }
}

fn asked_at(items: &[Item]) -> Option<Timestamp> {
    let markers: Vec<_> = items
        .iter()
        .filter_map(|item| match item.body {
            ItemBody::Interruption { asked_at, .. } => Some(asked_at),
            _ => None,
        })
        .collect();
    assert_eq!(markers.len(), 1);
    markers[0]
}

async fn agrees(kind: Option<InteractionKind>, close: bool) {
    let permission = matches!(&kind, Some(InteractionKind::Permission { .. }));
    let (mut actor, mut hooks) = observed_actor(permission).await;
    let turn = actor.running.as_ref().unwrap().turn.clone();
    let opened_at = if permission {
        Some(actor.state.interactions[0].opened_at)
    } else if let Some(kind) = kind {
        let (reply, _answer) = oneshot::channel();
        actor
            .open_interaction(None, kind, vec![AnswerSpec::Cancel], reply)
            .await
            .unwrap();
        Some(actor.state.interactions[0].opened_at)
    } else {
        None
    };
    if close {
        actor.close(CloseReason::Client).await.unwrap();
    } else {
        actor
            .interrupt(IntentId::mint(), InterruptScope::Head)
            .await
            .unwrap();
    }
    let outcome = returned(&mut actor).await;
    assert_eq!(asked_at(&outcome.items), opened_at, "turn outcome");
    actor
        .turn_finished(turn, Ended::Returned(outcome))
        .await
        .unwrap();
    while actor.state.turn.is_some() {
        let message = actor.rx.recv().await.unwrap();
        actor.handle(message).await;
    }
    assert_eq!(
        asked_at(&hooks.recv().await.unwrap()),
        opened_at,
        "turn-end hooks"
    );
    let items = actor
        .journal
        .iter()
        .filter_map(|frame| match &frame.event {
            Event::ItemCompleted { item } => Some(item.clone()),
            _ => None,
        })
        .collect::<Vec<_>>();
    assert_eq!(asked_at(&items), opened_at, "journal");
    assert!(actor.pending.is_empty());
}

fn permission() -> InteractionKind {
    serde_json::from_value(
        json!({"kind":"permission","tool":"Echo","input":{},"summary":"Allow Echo?"}),
    )
    .unwrap()
}

#[tokio::test]
async fn permission_stop_agrees_in_outcome_hooks_and_journal() {
    agrees(Some(permission()), false).await;
}
#[tokio::test]
async fn question_stop_agrees_in_outcome_hooks_and_journal() {
    agrees(
        Some(
            serde_json::from_value(json!({"kind":"question","question":"Where?",
        "options":[],"freeText":true,"multi":false}))
            .unwrap(),
        ),
        false,
    )
    .await;
}
#[tokio::test]
async fn stop_without_an_ask_agrees_in_outcome_hooks_and_journal() {
    agrees(None, false).await;
}
#[tokio::test]
async fn closing_with_an_ask_agrees_in_outcome_hooks_and_journal() {
    agrees(Some(permission()), true).await;
}

#[tokio::test]
async fn resumed_pending_calls_close_before_the_lost_turn() {
    let turn = TurnId::from_raw("lost");
    let mut frames = vec![
        journal_frame(
            1,
            Event::SessionUpdated {
                summary: summary("ses_1"),
            },
        ),
        journal_frame(
            2,
            Event::TurnStarted {
                turn: turn.clone(),
                inputs: vec![],
                origin: TurnOrigin::Submit,
            },
        ),
    ];
    for (index, status) in [ItemStatus::Running, ItemStatus::Pending]
        .into_iter()
        .enumerate()
    {
        let item = crate::turn::mint(
            &turn,
            0,
            ItemBody::ToolCall {
                call_id: format!("c{index}"),
                name: "Echo".into(),
                input: json!({}),
                output: None,
                duration_ms: None,
                progress: None,
                allowed_by: None,
                quiet: false,
            },
            status,
        );
        frames.push(journal_frame(index as u64 + 3, Event::ItemStarted { item }));
    }
    let mailbox = resume(frames, None, Services::none(), |_| {
        Arc::new(config(
            ScriptedProvider::new(vec![]),
            vec![],
            Arc::new(NoHost),
        ))
    })
    .unwrap();
    let (state, _) = mailbox.attach().await.unwrap();
    let calls: Vec<_> = state
        .transcript
        .items()
        .iter()
        .filter(|item| matches!(item.body, ItemBody::ToolCall { .. }))
        .collect();
    assert_eq!(calls.len(), 2);
    assert!(
        calls
            .iter()
            .all(|item| item.status == ItemStatus::Interrupted && item.completed_at.is_some())
    );
    let journal = events_of(&mailbox).await;
    let done = journal
        .iter()
        .position(|frame| matches!(frame.event, Event::TurnCompleted { .. }))
        .unwrap();
    assert_eq!(
        journal[..done]
            .iter()
            .filter(|frame| matches!(&frame.event,
        Event::ItemCompleted { item } if matches!(item.body, ItemBody::ToolCall { .. })))
            .count(),
        2
    );
    mailbox.close(CloseReason::Client);
    mailbox.wait_closed().await;
}
