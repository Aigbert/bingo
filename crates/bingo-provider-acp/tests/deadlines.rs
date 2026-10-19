//! Cancellation must outlive the dropped turn, and end an unresponsive child.
#![allow(clippy::unwrap_used, clippy::expect_used)]
mod harness;
#[path = "harness/prompt_host.rs"]
mod prompt_host;

use bingo_provider_acp::{agent::AcpAgent, session::Sessions};
use bingo_sdk::{Agent, Env, ModelRequest};
use futures::StreamExt;
use harness::{Fake, LIMIT, agent_binary};
use serde_json::json;
use std::time::Duration;
use tokio::io::AsyncReadExt;

fn request() -> ModelRequest {
    serde_json::from_value(json!({
        "model":"agent", "maxTokens":1, "system":[], "tools":[],
        "session":"ses_picture", "messages":[{"role":"user", "parts":[{"type":"text","text":"hello"}]}]
    })).unwrap()
}

async fn scenario(ignore: bool) {
    let fake = Fake::new(json!({"turns":[{"awaitCancel":true,"ignoreCancel":ignore}, {}]}));
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let mut env = fake.env();
    env.insert(
        "BINGO_FAKE_ACP_LIFETIME".into(),
        listener.local_addr().unwrap().to_string(),
    );
    let sessions = Sessions::new(Env::rooted(fake.cwd()));
    sessions.set_host(prompt_host::host(fake.cwd())).await;
    let adapter = serde_json::from_value(json!({"command":agent_binary(),"env":env})).unwrap();
    let provider = AcpAgent::new("scripted".into(), adapter, sessions.clone());
    let stream = provider.run(harness::turn(request())).await.unwrap();
    let (mut lifetime, _) = tokio::time::timeout(LIMIT, listener.accept())
        .await
        .unwrap()
        .unwrap();
    fake.wait_for("session/prompt").await;
    drop(stream);
    fake.wait_for("session/cancel").await;
    tokio::time::pause();
    tokio::time::advance(Duration::from_secs(9)).await;
    tokio::task::yield_now().await;
    let mut byte = [0];
    assert_eq!(
        lifetime.try_read(&mut byte).unwrap_err().kind(),
        std::io::ErrorKind::WouldBlock
    );
    tokio::time::advance(Duration::from_secs(2)).await;
    tokio::task::yield_now().await;
    tokio::time::resume();
    if ignore {
        let ended = tokio::time::timeout(LIMIT, lifetime.read(&mut byte))
            .await
            .expect("the ignored cancel must kill the child");
        // A killed process's socket closes on unix and is reset on Windows.
        assert!(
            matches!(&ended, Ok(0))
                || ended
                    .as_ref()
                    .is_err_and(|e| e.kind() == std::io::ErrorKind::ConnectionReset),
            "{ended:?}"
        );
        fake.rewrite(json!({"turns":[{}]}));
    }
    let mut next = tokio::time::timeout(LIMIT, provider.run(harness::turn(request())))
        .await
        .unwrap()
        .unwrap();
    while let Some(event) = tokio::time::timeout(LIMIT, next.next()).await.unwrap() {
        event.unwrap();
    }
    let children = fake
        .methods()
        .iter()
        .filter(|method| method.as_str() == "initialize")
        .count();
    assert_eq!(children, if ignore { 2 } else { 1 });
    sessions.close().await;
}

#[tokio::test]
async fn deadline_kills_an_agent_that_ignores_cancel_after_the_turn_is_dropped() {
    scenario(true).await;
}
#[tokio::test]
async fn deadline_keeps_an_agent_that_answers_cancel() {
    scenario(false).await;
}
