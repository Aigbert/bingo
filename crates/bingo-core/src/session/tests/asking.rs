//! Listings must see interactions without following the child stream.
use super::*;

async fn prompted() -> (Mailbox, SessionState, FrameStream) {
    let provider = ScriptedProvider::new(vec![
        Script::Events(tool_call("Echo", json!({"v": 1}))),
        Script::Events(text("finished")),
    ]);
    let mailbox = start(provider, vec![Arc::new(EchoTool { read_only: false })]);
    let (mut state, mut events) = mailbox.attach().await.unwrap();
    mailbox.submit(IntentId::mint(), Input::text("go", Origin::surface("test")));
    drive(&mut events, &mut state, |f| {
        matches!(f.event, Event::InteractionOpened { .. })
    })
    .await;
    // Mailbox order fences publication without waiting for a missing frame.
    assert!(
        mailbox.summary().await.unwrap().asking,
        "an open permission must stamp asking"
    );
    let frames = events_of(&mailbox).await;
    assert!(
        frames
            .iter()
            .any(|f| matches!(&f.event, Event::SessionUpdated { summary } if summary.asking)),
        "opening must publish the stamp"
    );
    (mailbox, state, events)
}

#[tokio::test]
async fn an_answer_republishes_not_asking() {
    let (mailbox, mut state, mut events) = prompted().await;
    mailbox.answer(
        IntentId::mint(),
        state.interactions[0].id.clone(),
        Answer::AllowOnce,
        Activation::Programmatic,
        who(),
    );
    drive(&mut events, &mut state, turn_completed).await;
    assert!(!mailbox.summary().await.unwrap().asking);
    assert!(!state.summary.asking);
}

#[tokio::test]
async fn esc_republishes_not_asking() {
    let (mailbox, mut state, mut events) = prompted().await;
    mailbox.interrupt(IntentId::mint(), InterruptScope::Head);
    drive(&mut events, &mut state, turn_completed).await;
    assert!(!mailbox.summary().await.unwrap().asking);
    assert!(!state.summary.asking);
}

#[tokio::test]
async fn a_receipt_queued_after_interruption_cannot_publish_under_the_closed_turn() {
    let (mailbox, mut state, mut events) = prompted().await;
    let mut item = state.transcript.items().last().unwrap().as_ref().clone();
    item.id = ItemId::mint();
    item.body = ItemBody::PermissionReceipt {
        item: None,
        interaction: state.interactions[0].id.clone(),
        tool: "Echo".into(),
        decision: DecisionKind::Allow,
        feedback: None,
    };
    let id = item.id.clone();
    mailbox.interrupt(IntentId::mint(), InterruptScope::Head);
    drive(&mut events, &mut state, turn_completed).await;
    let _ = mailbox
        .call(|reply| mailbox::Msg::Receipt { item, reply })
        .await
        .unwrap();
    let frames = events_of(&mailbox).await;
    assert!(
        !frames
            .iter()
            .any(|frame| matches!(&frame.event, Event::ItemCompleted { item } if item.id == id))
    );
}
