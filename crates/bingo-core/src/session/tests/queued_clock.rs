use super::*;

#[tokio::test]
async fn a_queued_messages_clock_starts_when_its_turn_lands() {
    let mailbox = start(
        ScriptedProvider::new(vec![Script::Hang(vec![]), Script::Hang(vec![])]),
        vec![],
    );
    let (mut state, mut events) = mailbox.attach().await.unwrap();
    mailbox.submit(
        IntentId::mint(),
        Input::text("first", Origin::surface("test")),
    );
    drive(&mut events, &mut state, |f| {
        matches!(f.event, Event::TurnStarted { .. })
    })
    .await;
    let intent = IntentId::mint();
    mailbox.submit(
        intent.clone(),
        Input::text("queued", Origin::surface("test")),
    );
    let queued = frames_until(&mut events, &mut state, |f|
        matches!(&f.event, Event::IntentAck { intent: id, outcome: IntentOutcome::Queued { .. } } if id == &intent)).await;
    let queued_at = queued.last().unwrap().ts;
    while Timestamp::now() <= queued_at {
        tokio::task::yield_now().await;
    }
    mailbox.interrupt(IntentId::mint(), InterruptScope::Head);
    let first = frames_until(&mut events, &mut state, turn_completed).await;
    let first_ended = first.last().unwrap().ts;
    let second = frames_until(&mut events, &mut state, |f| {
        matches!(f.event, Event::TurnStarted { .. })
    })
    .await;
    let landed = state
        .transcript
        .items()
        .iter()
        .find(|i| i.intent.as_ref() == Some(&intent))
        .unwrap();
    assert!(landed.started_at >= first_ended);
    assert!(landed.started_at > queued_at);
    let now = second.last().unwrap().ts;
    let clock = bingo_sdk::transcript::clock(&state, landed.turn.as_ref().unwrap(), now).unwrap();
    assert_eq!(clock.elapsed, now.duration_since(landed.started_at));
    assert!(clock.elapsed < now.duration_since(queued_at));
    mailbox.close(CloseReason::Client);
}
