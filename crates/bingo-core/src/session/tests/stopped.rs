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

async fn observed_actor() -> (Actor, mpsc::UnboundedReceiver<Vec<Item>>) {
    let (mut actor, _) = storage::failing_actor().await;
    let (send, receive) = mpsc::unbounded_channel();
    let mut cfg = config(
        ScriptedProvider::new(vec![Script::Hang(vec![])]),
        vec![],
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
    let (mut actor, mut hooks) = observed_actor().await;
    let turn = actor.running.as_ref().unwrap().turn.clone();
    let opened_at = if let Some(kind) = kind {
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
