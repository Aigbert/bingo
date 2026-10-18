use super::*;

struct Asking;
#[async_trait::async_trait]
impl PermissionPolicy for Asking {
    fn id(&self) -> &str {
        "fixture"
    }
    fn allows_replacement(&self, _: PolicyInput<'_>) -> bool {
        true
    }
    async fn decide(&self, _: PolicyInput<'_>) -> Decision {
        Decision::Ask {
            reason: Reason::Default,
            scope: Some("Echo".into()),
        }
    }
}

async fn prompted() -> (Mailbox, SessionState, FrameStream) {
    let provider = ScriptedProvider::new(vec![
        Script::Events(tool_call("Echo", json!({"v":1}))),
        Script::Events(text("done")),
    ]);
    let mut cfg = config(
        provider,
        vec![Arc::new(EchoTool { read_only: false })],
        Arc::new(NoHost),
    );
    cfg.policy = Arc::new(Asking);
    let mailbox = spawn(summary("ses_1"), None, Services::none(), |_| Arc::new(cfg));
    let (mut state, mut events) = mailbox.attach().await.unwrap();
    mailbox.submit(IntentId::mint(), Input::text("go", Origin::surface("test")));
    drive(&mut events, &mut state, opened).await;
    (mailbox, state, events)
}
fn opened(frame: &Frame) -> bool {
    matches!(frame.event, Event::InteractionOpened { .. })
}
fn answer(mailbox: &Mailbox, state: &SessionState, answer: Answer) {
    mailbox.answer(
        IntentId::mint(),
        state.interactions[0].id.clone(),
        answer,
        Activation::Programmatic,
        who(),
    );
}
fn receipts(state: &SessionState) -> Vec<&ItemBody> {
    state
        .transcript
        .items()
        .iter()
        .filter_map(|item| {
            matches!(item.body, ItemBody::PermissionReceipt { .. }).then_some(&item.body)
        })
        .collect()
}
async fn no_corrections(mailbox: &Mailbox) {
    assert!(!events_of(mailbox).await.iter().any(|f| matches!(&f.event,
        Event::ItemUpdated { item } if matches!(item.body, ItemBody::PermissionReceipt { .. }))));
}

#[tokio::test]
async fn mismatched_session_scope_records_only_the_final_denial() {
    let (mailbox, mut state, mut events) = prompted().await;
    let ask = state.interactions[0].clone();
    answer(
        &mailbox,
        &state,
        Answer::AllowSession {
            scope: "Other".into(),
        },
    );
    drive(&mut events, &mut state, turn_completed).await;
    assert_eq!(
        receipts(&state),
        vec![&ItemBody::PermissionReceipt {
            item: ask.item,
            interaction: ask.id,
            tool: "Echo".into(),
            decision: DecisionKind::Deny,
            feedback: None,
            asked_at: Some(ask.opened_at)
        }]
    );
    no_corrections(&mailbox).await;
}

#[tokio::test]
async fn edited_then_session_allowed_records_one_final_interaction() {
    let (mailbox, mut state, mut events) = prompted().await;
    let first = state.interactions[0].clone();
    answer(
        &mailbox,
        &state,
        Answer::AllowChanged {
            input: json!({"v":2}),
        },
    );
    drive(&mut events, &mut state, opened).await;
    let last = state.interactions[0].clone();
    answer(
        &mailbox,
        &state,
        Answer::AllowSession {
            scope: "Echo".into(),
        },
    );
    drive(&mut events, &mut state, turn_completed).await;
    assert_ne!(first.id, last.id);
    assert_eq!(
        receipts(&state),
        vec![&ItemBody::PermissionReceipt {
            item: last.item,
            interaction: last.id,
            tool: "Echo".into(),
            decision: DecisionKind::AllowChangedSession,
            feedback: None,
            asked_at: Some(first.opened_at)
        }]
    );
    no_corrections(&mailbox).await;
}

#[tokio::test]
async fn denial_feedback_survives_an_immediate_escape_once() {
    let (mailbox, mut state, mut events) = prompted().await;
    let ask = state.interactions[0].clone();
    answer(
        &mailbox,
        &state,
        Answer::Deny {
            feedback: Some("use a smaller change".into()),
        },
    );
    mailbox.interrupt(IntentId::mint(), InterruptScope::Head);
    drive(&mut events, &mut state, turn_completed).await;
    assert_eq!(
        receipts(&state),
        vec![&ItemBody::PermissionReceipt {
            item: ask.item,
            interaction: ask.id,
            tool: "Echo".into(),
            decision: DecisionKind::Deny,
            feedback: Some("use a smaller change".into()),
            asked_at: Some(ask.opened_at)
        }]
    );
}

async fn questioning() -> (Mailbox, SessionState, FrameStream) {
    let mailbox = start(ScriptedProvider::new(vec![Script::Hang(vec![])]), vec![]);
    let (mut state, mut events) = mailbox.attach().await.unwrap();
    mailbox.submit(IntentId::mint(), Input::text("go", Origin::surface("test")));
    drive(&mut events, &mut state, |f| {
        matches!(f.event, Event::TurnStarted { .. })
    })
    .await;
    tokio::spawn({
        let mailbox = mailbox.clone();
        async move {
            mailbox
                .ask(
                    None,
                    serde_json::from_value(json!({"kind":"question","question":"Where?",
            "options":[],"freeText":true,"multi":false}))
                    .unwrap(),
                    vec![AnswerSpec::Text],
                )
                .await
        }
    });
    drive(&mut events, &mut state, opened).await;
    (mailbox, state, events)
}

async fn stopped(question: bool) {
    let (mailbox, mut state, mut events) = if question {
        questioning().await
    } else {
        prompted().await
    };
    let ask = state.interactions[0].clone();
    mailbox.interrupt(IntentId::mint(), InterruptScope::Head);
    drive(&mut events, &mut state, turn_completed).await;
    assert!(state.transcript.items().iter().all(|i| !matches!(
        i.body,
        ItemBody::PermissionReceipt { .. } | ItemBody::QuestionAnswer { .. }
    )));
    let model =
        serde_json::to_string(&crate::context::ContextView::model_view(&state.transcript)).unwrap();
    assert!(!model.contains("skipped") && !model.contains("Where?"));
    let turn = ask.turn.unwrap();
    let start = state
        .transcript
        .items()
        .iter()
        .find(|i| i.turn.as_ref() == Some(&turn))
        .unwrap()
        .started_at;
    let clock = bingo_sdk::transcript::clock(&state, &turn, Timestamp::now()).unwrap();
    let wait_start = if question {
        ask.opened_at
    } else {
        bingo_sdk::transcript::call_wait(state.transcript.get(ask.item.as_ref().unwrap()).unwrap())
            .unwrap()
            .0
    };
    assert_eq!(clock.elapsed, wait_start.duration_since(start));
    let interruption = state
        .transcript
        .items()
        .iter()
        .find(|i| matches!(i.body, ItemBody::Interruption { .. }))
        .unwrap();
    assert_eq!(
        serde_json::to_value(&interruption.body).unwrap()["askedAt"],
        json!(ask.opened_at)
    );
}
#[tokio::test]
async fn stopping_a_permission_does_not_invent_an_answer() {
    stopped(false).await;
}
#[tokio::test]
async fn stopping_a_question_does_not_invent_an_answer() {
    stopped(true).await;
}

#[tokio::test]
async fn two_answered_asks_keep_their_own_opening_instants() {
    let provider = ScriptedProvider::new(vec![
        Script::Events(tool_call("Echo", json!({"v":1}))),
        Script::Events(tool_call("Echo", json!({"v":2}))),
        Script::Events(text("done")),
    ]);
    let mailbox = start(provider, vec![Arc::new(EchoTool { read_only: false })]);
    let (mut state, mut events) = mailbox.attach().await.unwrap();
    mailbox.submit(IntentId::mint(), Input::text("go", Origin::surface("test")));
    let mut asks = Vec::new();
    for _ in 0..2 {
        let frames = frames_until(&mut events, &mut state, opened).await;
        let Event::InteractionOpened { interaction } = &frames.last().unwrap().event else {
            panic!("opened frame");
        };
        asks.push(interaction.clone());
        answer(&mailbox, &state, Answer::AllowOnce);
    }
    drive(&mut events, &mut state, turn_completed).await;
    assert_ne!(asks[0].opened_at, asks[1].opened_at);
    let receipts = receipts(&state);
    assert_eq!(receipts.len(), 2);
    for ask in asks {
        let matching: Vec<_> = receipts
            .iter()
            .filter_map(|body| match body {
                ItemBody::PermissionReceipt {
                    interaction,
                    asked_at,
                    ..
                } if interaction == &ask.id => Some(*asked_at),
                _ => None,
            })
            .collect();
        assert_eq!(matching, vec![Some(ask.opened_at)]);
    }
}
