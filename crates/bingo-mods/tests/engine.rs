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
            // The budget bounds registration too: room for a loaded runner to
            // load the module before the loop is the one that runs it out.
            hook_time: Duration::from_secs(1),
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

#[tokio::test]
async fn an_unawaited_dollar_call_finishes_before_the_hook_returns() {
    let root = tempfile::tempdir().unwrap();
    std::fs::write(root.path().join("main.ts"), "export const register = on => on('tool.call', $ => { void $.ui.status('queued'); return {parts:[]}; });").unwrap();
    let engine = Engine::start(root.path(), "main.ts", json!({}), Limits::default())
        .await
        .unwrap();
    let mut call = engine
        .call("tool.call", json!({}), json!({"session":"s"}))
        .unwrap();
    let operation = tokio::select! {
        biased;
        result = &mut call.result => panic!("hook returned before its dollar call: {result:?}"),
        operation = call.operations.recv() => operation.unwrap(),
    };
    assert_eq!(operation.method, "host/place");
    tokio::task::yield_now().await;
    assert!(matches!(
        call.result.try_recv(),
        Err(tokio::sync::oneshot::error::TryRecvError::Empty)
    ));
    operation.reply.send(Ok(Value::Null)).unwrap();
    assert_eq!(
        (&mut call.result).await.unwrap().unwrap(),
        json!({"parts":[]})
    );
    engine.close().await;
}

#[tokio::test]
async fn retired_turn_sites_refuse_loading_in_words() {
    let root = tempfile::tempdir().unwrap();
    for (site, matcher) in [
        ("TurnFold", "'TurnFold'"),
        ("TurnFold", "['Ledger', 'TurnFold']"),
        ("TurnProgress", "'TurnProgress'"),
        ("TurnProgress", "['Spinner', 'TurnProgress']"),
    ] {
        std::fs::write(root.path().join("main.js"), format!(
            "export function register(on) {{ on('ui.render', {{component:{matcher}}}, ($, e, next) => next(e)); }}"
        )).unwrap();
        let result = Engine::start(root.path(), "main.js", json!({}), Limits::default()).await;
        let error = result
            .err()
            .expect("a retired site cannot load")
            .to_string();
        assert!(error.contains(site) && error.contains("retired"), "{error}");
    }
}
