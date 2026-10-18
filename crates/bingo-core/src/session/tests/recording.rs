use super::*;
use std::sync::atomic::{AtomicUsize, Ordering};
use tokio::sync::oneshot;

struct Described(Arc<AtomicUsize>);
#[async_trait::async_trait]
impl PermissionPolicy for Described {
    fn id(&self) -> &str {
        "counted"
    }
    fn describe(&self, _: &SessionId) -> Value {
        json!(self.0.fetch_add(1, Ordering::SeqCst))
    }
    async fn decide(&self, _: PolicyInput<'_>) -> Decision {
        Decision::Allow {
            reason: Reason::ReadOnly,
        }
    }
}

async fn actor() -> (Actor, Arc<AtomicUsize>) {
    let (mut actor, _) = storage::failing_actor().await;
    let count = Arc::new(AtomicUsize::new(0));
    let mut cfg = config(
        ScriptedProvider::new(vec![Script::Hang(vec![])]),
        vec![],
        Arc::new(NoHost),
    );
    cfg.policy = Arc::new(Described(count.clone()));
    actor.config = Arc::new(cfg);
    actor
        .start_turn(
            vec![(IntentId::mint(), Input::text("go", Origin::surface("test")))],
            TurnOrigin::Submit,
            TurnKind::Respond,
        )
        .await
        .unwrap();
    (actor, count)
}

async fn ask(actor: &mut Actor) -> Interaction {
    let (reply, _answer) = oneshot::channel();
    actor
        .open_interaction(
            None,
            serde_json::from_value(json!({
                "kind":"permission","tool":"Echo","input":{},"summary":"Allow Echo?"
            }))
            .unwrap(),
            vec![AnswerSpec::Deny],
            reply,
        )
        .await
        .unwrap();
    actor.state.interactions[0].clone()
}

fn receipt(actor: &Actor, interaction: InteractionId) -> Item {
    actor.fresh(
        actor.running.as_ref().map(|r| r.turn.clone()),
        None,
        ItemBody::PermissionReceipt {
            item: None,
            interaction,
            tool: "Echo".into(),
            decision: DecisionKind::Deny,
            feedback: None,
            asked_at: None,
        },
    )
}

#[derive(Clone, Copy)]
enum Path {
    Emit,
    Record,
    Receipt,
}

async fn record(actor: &mut Actor, path: Path, item: Item) {
    match path {
        Path::Emit => actor
            .emit(item.turn.clone().unwrap(), Event::ItemCompleted { item })
            .await
            .unwrap(),
        Path::Record => {
            actor.record_for(None, item.body).await.unwrap();
        }
        Path::Receipt => {
            let (reply, answer) = oneshot::channel();
            actor.receipt(item, reply).await.unwrap();
            answer.await.unwrap().unwrap();
        }
    }
}

async fn refreshes(path: Path) {
    let (mut actor, count) = actor().await;
    let item = receipt(&actor, InteractionId::mint());
    let before = count.load(Ordering::SeqCst);
    record(&mut actor, path, item.clone()).await;
    assert_eq!(count.load(Ordering::SeqCst), before + 1, "new receipt");
    record(&mut actor, path, item.clone()).await;
    assert_eq!(
        count.load(Ordering::SeqCst),
        before + 2,
        "duplicate receipt"
    );
    let receipts: Vec<_> = actor
        .state
        .transcript
        .items()
        .iter()
        .filter(|i| matches!(i.body, ItemBody::PermissionReceipt { .. }))
        .collect();
    assert_eq!(receipts.len(), 1);
    assert_eq!(receipts[0].turn, item.turn);
    actor.running.take().unwrap().abandon();
}

#[tokio::test]
async fn emitted_receipts_refresh_once_even_when_already_recorded() {
    refreshes(Path::Emit).await;
}
#[tokio::test]
async fn command_receipts_refresh_once_even_when_already_recorded() {
    refreshes(Path::Record).await;
}
#[tokio::test]
async fn gate_receipts_refresh_once_even_when_already_recorded() {
    refreshes(Path::Receipt).await;
}

#[tokio::test]
async fn an_answered_denial_refreshes_once() {
    let (mut actor, count) = actor().await;
    let opened = ask(&mut actor).await;
    let before = count.load(Ordering::SeqCst);
    actor
        .answer(super::super::mailbox::Answered {
            intent: IntentId::mint(),
            interaction: opened.id,
            answer: Answer::Deny { feedback: None },
            activation: Activation::Programmatic,
            who: who(),
            plugin: false,
        })
        .await
        .unwrap();
    assert_eq!(count.load(Ordering::SeqCst), before + 1);
    let receipt = actor
        .state
        .transcript
        .items()
        .iter()
        .find(|item| matches!(item.body, ItemBody::PermissionReceipt { .. }))
        .unwrap();
    assert_eq!(receipt.turn, opened.turn);
    actor.running.take().unwrap().abandon();
}

#[tokio::test]
async fn publishing_does_not_rewrite_an_interruption() {
    let (mut actor, _) = actor().await;
    let opened = ask(&mut actor).await;
    actor.remember_stopped_ask();
    actor
        .cancel_interactions(CancelReason::Interrupted)
        .await
        .unwrap();
    let item = actor.fresh(
        opened.turn,
        None,
        ItemBody::Interruption {
            marker: "stopped".into(),
            asked_at: None,
        },
    );
    actor
        .publish(Event::ItemCompleted { item: item.clone() }, None)
        .await
        .unwrap();
    assert_eq!(actor.state.transcript.get(&item.id).unwrap(), &item);
    assert_eq!(
        actor.running.as_ref().unwrap().stopped_ask.get(),
        Some(opened.opened_at)
    );
    actor.running.take().unwrap().abandon();
}

#[tokio::test]
async fn a_dropped_turn_closes_its_open_ask() {
    let (mut actor, _) = actor().await;
    let opened = ask(&mut actor).await;
    let turn = opened.turn.clone().unwrap();
    actor
        .turn_finished(turn.clone(), Ended::Dropped)
        .await
        .unwrap();
    let marker = actor
        .state
        .transcript
        .items()
        .iter()
        .find(|i| matches!(i.body, ItemBody::Interruption { .. }))
        .unwrap();
    assert!(
        matches!(marker.body, ItemBody::Interruption { asked_at: Some(at), .. } if at == opened.opened_at)
    );
    assert_eq!(
        actor.running.as_ref().unwrap().stopped_ask.get(),
        Some(opened.opened_at)
    );
    // Completion is queued through the catalogue even without any interceptors.
    while actor.state.turn.is_some() {
        let message = actor.rx.recv().await.unwrap();
        actor.handle(message).await;
    }
    assert!(actor.state.turn.is_none());
    let start = actor.state.transcript.items()[0].started_at;
    let clock = bingo_sdk::transcript::clock(&actor.state, &turn, Timestamp::now()).unwrap();
    assert_eq!(clock.elapsed, opened.opened_at.duration_since(start));
    assert_eq!(clock.next_second, None);
    assert!(!actor.state.transcript.items().iter().any(|i| matches!(
        i.body,
        ItemBody::PermissionReceipt { .. } | ItemBody::QuestionAnswer { .. }
    )));
}

#[tokio::test]
async fn a_receipt_without_a_turn_is_refused_but_record_for_works_between_turns() {
    let (mut actor, _) = storage::failing_actor().await;
    let item = receipt(&actor, InteractionId::mint());
    let (reply, answer) = oneshot::channel();
    actor.receipt(item.clone(), reply).await.unwrap();
    assert_eq!(
        answer.await.unwrap().unwrap_err().code,
        ErrorCode::Interrupted
    );
    assert!(actor.state.transcript.items().is_empty());
    actor.record_for(None, item.body).await.unwrap();
    assert_eq!(actor.state.transcript.items().len(), 1);
}
