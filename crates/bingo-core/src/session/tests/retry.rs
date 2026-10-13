use super::*;
use crate::journal::MemoryStore;

fn retry_script() -> Arc<ScriptedProvider> {
    ScriptedProvider::new(vec![
        Script::Events(vec![
            Ok(ModelEvent::TextStart { id: "b".into() }),
            Ok(ModelEvent::TextDelta {
                id: "b".into(),
                delta: "withdrawn text".into(),
            }),
            Err(ProviderError::Server {
                status: 503,
                message: "overloaded".into(),
            }),
        ]),
        Script::Events(text("successful retry")),
        Script::Events(text("next answer")),
        Script::Events(text("resumed answer")),
    ])
}

async fn ask(mailbox: &Mailbox, prompt: &str) {
    let (mut state, mut events) = mailbox.attach().await.unwrap();
    mailbox.submit(
        IntentId::mint(),
        Input::text(prompt, Origin::surface("test")),
    );
    drive(&mut events, &mut state, turn_completed).await;
    assert_eq!(state.last_status(), Some(&TurnStatus::Completed));
}

async fn retry_requests() -> Vec<ModelRequest> {
    let provider = retry_script();
    let store = Arc::new(MemoryStore::new());
    store.create(&summary("ses_1")).await.unwrap();
    let mailbox = spawn(
        summary("ses_1"),
        Some(store.clone()),
        Services::none(),
        |_| Arc::new(config(provider.clone(), vec![], Arc::new(NoHost))),
    );
    ask(&mailbox, "first").await;
    ask(&mailbox, "second").await;
    mailbox.close(CloseReason::Client);
    mailbox.wait_closed().await;
    let journal = store.replay(mailbox.id(), Seq::ZERO).await.unwrap();
    let resumed = resume(journal, Some(store), Services::none(), |_| {
        Arc::new(config(provider.clone(), vec![], Arc::new(NoHost)))
    })
    .unwrap();
    ask(&resumed, "third").await;
    resumed.close(CloseReason::Client);
    resumed.wait_closed().await;
    provider.requests()
}

fn assert_clean(request: &ModelRequest) {
    let messages = serde_json::to_string(&request.messages).unwrap();
    assert!(messages.contains("successful retry"));
    assert!(!messages.contains("withdrawn text"), "{messages}");
}

#[tokio::test(start_paused = true)]
async fn retry_withdrawal_survives_the_next_turn() {
    assert_clean(&retry_requests().await[2]);
}

#[tokio::test(start_paused = true)]
async fn retry_withdrawal_survives_resume() {
    assert_clean(&retry_requests().await[3]);
}
