#![allow(clippy::unwrap_used)]
use bingo_mods::engine::{Engine, Fault, Limits};
use serde_json::{Value, json};
use std::{path::Path, sync::Arc, time::Duration};

async fn load(module: &str, limits: Limits) -> Arc<Engine> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/mods");
    Engine::start(&root, module, json!({}), limits)
        .await
        .unwrap()
}
async fn call(engine: &Engine, event: &str) -> Result<Value, Fault> {
    let mut call = engine
        .call(
            event,
            json!({"tool":"Read","tool_use_id":"1"}),
            json!({"session":"s","origin":{"plugin":"test","tier":"rewrite"}}),
        )
        .unwrap();
    loop {
        tokio::select! {
            result = &mut call.result => return result.unwrap(),
            Some(operation) = call.operations.recv() => {
                assert_eq!(operation.method, "next");
                operation.reply.send(Ok(json!({"parts":[{"kind":"text","text":"below"}]}))).unwrap();
            }
        }
    }
}

#[tokio::test]
async fn a_fixture_answers_on_its_own_runtime() {
    let engine = load("hello.ts", Limits::default()).await;
    assert_eq!(engine.patterns(), ["tool.call"]);
    assert_eq!(
        call(&engine, "tool.call").await.unwrap()["parts"][0]["text"],
        "hello"
    );
    engine.close().await;
}

#[tokio::test]
async fn catch_observes_a_throw_and_can_continue() {
    let engine = load("thrower.ts", Limits::default()).await;
    assert_eq!(
        call(&engine, "tool.call").await.unwrap()["parts"][0]["text"],
        "below"
    );
    engine.close().await;
}

#[tokio::test]
async fn a_loop_is_interrupted_on_the_module_thread() {
    let engine = load(
        "loop.ts",
        Limits {
            hook_time: Duration::from_millis(50),
            ..Limits::default()
        },
    )
    .await;
    assert_eq!(
        call(&engine, "tool.check").await.unwrap_err().kind,
        "timeout"
    );
    engine.close().await;
}

#[tokio::test]
async fn an_allocation_failure_leaves_the_runtime_available() {
    let engine = load(
        "hog.ts",
        Limits {
            memory_bytes: 4 * 1024 * 1024,
            ..Limits::default()
        },
    )
    .await;
    let fault = call(&engine, "tool.call").await.unwrap_err();
    assert!(fault.message.to_lowercase().contains("memory"), "{fault}");
    assert_eq!(
        call(&engine, "other.event").await.unwrap()["parts"][0]["text"],
        "below"
    );
    engine.close().await;
}
