//! Interceptor phases and observation notifications through a real child.
use crate::harness::{Started, hook_context, started_with};
use bingo_sdk::*;
use futures::FutureExt;
use serde_json::{Value, json};
use std::sync::Arc;
async fn with_hooks() -> Started {
    started_with(&[("stub", &[])]).await
}
async fn hook_of(manager: &bingo_plugin_rpc::Manager, id: &str) -> Arc<dyn Hook> {
    manager
        .hooks()
        .await
        .into_iter()
        .find(|hook| hook.id() == format!("stub:{id}"))
        .unwrap()
}
async fn link_of(started: &Started, point: LinkPoint) -> Arc<dyn Interceptor> {
    started
        .manager
        .interceptors()
        .await
        .into_iter()
        .find(|link| link.link.matcher().matches(point, &serde_json::Value::Null))
        .expect("the handshake declares this link")
        .link
}
/// What the process wrote down about the crossings it saw, read through its
/// own service. `null` is a process nothing has reached.
async fn crossings(started: &Started) -> Value {
    let kv = started
        .host
        .service::<ServiceHandle>(&bingo_sdk::ServiceKey::new("kv"))
        .expect("the stub serves kv");
    kv.call("get", json!({ "key": "hooks" }))
        .await
        .expect("the read crossed")
}

/// An observation crosses as a notification: the process answers it with
/// nothing, and the call still returns — a lane that waited on an answer
/// would never come back here. That it arrived is read afterwards, on the
/// same pipe, so the ordering is the pipe's rather than a sleep's.
#[tokio::test]
async fn an_observation_point_lands_as_a_notification_nobody_waits_on() {
    let started = with_hooks().await;
    let hook = hook_of(&started.manager, "watch").await;
    hook.on_session(Phase::Start, &hook_context()).await;
    hook.on_turn(
        Phase::End,
        &bingo_sdk::TurnId::from_raw("trn_test"),
        &[],
        &hook_context(),
    )
    .await;
    assert_eq!(crossings(&started).await, json!("watch/session,watch/turn"));
    assert!(
        started.said().is_empty(),
        "nothing was awaited, so no deadline was missed"
    );
    started.manager.shutdown().await;
}

#[tokio::test]
async fn a_remote_tool_link_sees_its_result_after_next() {
    let started = with_hooks().await;
    let link = link_of(&started, bingo_sdk::catalogue::TOOL_CALL).await;
    let next = Next::at(Tier::Rewrite, |call: ToolCall| {
        async move {
            assert_eq!(call.name, "Bash");
            assert_eq!(call.call_id, "c1");
            assert_eq!(call.input["command"], "ls --safe");
            Ok(ToolOutput::text("kernel"))
        }
        .boxed()
    });
    let result = link
        .tool_call(
            &hook_context(),
            ToolCall {
                name: "Bash".into(),
                call_id: "c1".into(),
                input: json!({"command":"ls"}),
            },
            next,
        )
        .await
        .unwrap();
    assert_eq!(result, ToolOutput::text("kernel (stub)"));
    assert_eq!(
        crossings(&started).await,
        json!("tool.call/before,tool.call/after")
    );
    started.manager.shutdown().await;
}

#[tokio::test]
async fn dropping_a_first_phase_notifies_the_process() {
    let started = with_hooks().await;
    let link = link_of(&started, bingo_sdk::catalogue::TURN_STOP).await;
    let cx = hook_context();
    let next = Next::new(|()| async { panic!("first phase never answers") }.boxed());
    let mut call = Box::pin(link.turn_stop(&cx, (), next));
    assert!(futures::poll!(&mut call).is_pending());
    drop(call);
    assert_eq!(
        crossings(&started).await,
        json!("turn.stop/before,chain/cancel")
    );
    started.manager.shutdown().await;
}
