//! The two things ADR-0015 opened: a tool and a command, run in another
//! process and answered here.

use std::sync::Arc;
use std::time::Duration;

use bingo_plugin_rpc::Manager;
use bingo_sdk::{CancellationToken, CommandContext, CommandOutcome, SessionId};
use serde_json::json;

use crate::harness::{CALL_ID, Recorder, call, context, only_tool, said, started};

#[tokio::test]
async fn a_plugin_s_tools_are_named_for_it_and_are_untrusted() {
    let (manager, _home, _project) = started(&[]).await;
    let tool = only_tool(&manager).await;
    let spec = tool.spec();
    assert_eq!(spec.name, "plugin__stub__echo");
    assert_eq!(spec.meta["plugin"], json!("stub"));
    assert_eq!(
        serde_json::to_value(spec.look).unwrap(),
        json!({"mark":"/", "verb":"echo"})
    );
    let traits = tool.traits(&json!({}));
    assert!(
        !traits.trusted,
        "nothing a process says about itself is a fact"
    );
    assert!(!traits.read_only && !traits.concurrency_safe);
    manager.shutdown().await;
}

#[tokio::test]
async fn a_plugin_s_commands_keep_the_name_the_plugin_gave_them() {
    let (manager, _home, project) = started(&[]).await;
    let commands = manager.commands().await;
    assert_eq!(commands.len(), 1);
    assert_eq!(commands[0].spec().name, "stub");
    let cx = CommandContext {
        cancel: bingo_sdk::CancellationToken::new(),
        session: SessionId::from_raw("ses_test"),
        cwd: project.path().to_path_buf(),
        host: bingo_sdk::testing::NoHost::handle(),
    };
    let outcome = commands[0].run("two words", &cx).await.expect("it ran");
    let CommandOutcome::Applied { message } = outcome else {
        panic!("the stub answers with an applied outcome");
    };
    let message = message.expect("with a message");
    assert!(message.starts_with("stub in "), "{message}");
    assert!(message.ends_with(": two words"), "{message}");
    manager.shutdown().await;
}

#[tokio::test]
async fn a_call_crosses_the_pipe_and_the_output_comes_back() {
    let (manager, _home, project) = started(&[]).await;
    let tool = only_tool(&manager).await;
    let (_, answered) = call(&tool, json!({ "text": "hello" }), project.path()).await;
    assert_eq!(said(&answered.expect("an output")), "hello");
    manager.shutdown().await;
}

#[tokio::test]
async fn the_plugin_root_reaches_the_process_through_its_environment() {
    let (manager, home, project) = started(&[]).await;
    let tool = only_tool(&manager).await;
    let (_, answered) = call(&tool, json!({ "env": "PLUGIN_HOME" }), project.path()).await;
    assert_eq!(
        said(&answered.expect("an output")),
        home.path()
            .join(".bingo/plugins/stub")
            .display()
            .to_string()
    );
    manager.shutdown().await;
}

#[tokio::test]
async fn a_progress_notification_becomes_the_call_s_live_output_line() {
    let (manager, _home, project) = started(&[]).await;
    let tool = only_tool(&manager).await;
    let (recorder, answered) = call(
        &tool,
        json!({ "progress": ["reading", "counting"], "text": "done" }),
        project.path(),
    )
    .await;
    assert_eq!(said(&answered.expect("an output")), "done");
    assert_eq!(recorder.progress(), ["reading", "counting"]);
    manager.shutdown().await;
}

/// The calls this process was told to stop, as it read them off the pipe. A
/// drop cannot wait for the notice it sends, so a test that asks about one
/// polls.
async fn cancelled(manager: &Manager, cwd: &std::path::Path) -> String {
    for _ in 0..300 {
        let tool = only_tool(manager).await;
        let (_, answered) = call(&tool, json!({ "cancelled": true }), cwd).await;
        let seen = said(&answered.expect("an output"));
        if !seen.is_empty() {
            return seen;
        }
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
    panic!("the process was never told to stop");
}

/// An interrupt is the kernel letting the call go where it stands: nobody
/// reads the answer any more, and `tool/cancel` goes out from the drop. What
/// the process then does about it is the process's own affair — it may still
/// be running the call, and nothing here waits to find out.
#[tokio::test]
async fn a_dropped_call_tells_the_plugin_it_is_no_longer_waited_for() {
    let (manager, _home, project) = started(&[]).await;
    let tool = only_tool(&manager).await;
    let recorder = Arc::new(Recorder::default());
    let cx = context(recorder, project.path(), CancellationToken::new());
    // Boxed rather than pinned in place: this test's whole point is letting
    // the future go, and a `tokio::pin!` handle drops the borrow, not the
    // future.
    let mut running = Box::pin(tool.call(json!({ "awaitCancel": true }), &cx));
    // The call is in flight and the stub is holding it: nothing answers it.
    let held = tokio::time::timeout(Duration::from_millis(200), &mut running).await;
    assert!(
        held.is_err(),
        "the stub answers nothing until it is cancelled"
    );
    drop(running);
    assert_eq!(cancelled(&manager, project.path()).await, CALL_ID);
    manager.shutdown().await;
}

async fn deadline_tool(seconds: Option<u64>) {
    let started = crate::harness::started_with_timeout(&[("stub", &[])], seconds).await;
    let tool = only_tool(&started.manager).await;
    let cx = context(
        Arc::new(Recorder::default()),
        started.project.path(),
        CancellationToken::new(),
    );
    tokio::time::pause();
    let mut running = Box::pin(tool.call(json!({"awaitCancel": true}), &cx));
    assert!(futures::poll!(&mut running).is_pending());
    let bound = Duration::from_secs(seconds.unwrap_or(600));
    tokio::time::advance(bound - Duration::from_secs(1)).await;
    assert!(
        futures::poll!(&mut running).is_pending(),
        "the deadline must not fire early"
    );
    let result = tokio::time::timeout(Duration::from_secs(2), &mut running).await;
    drop(running);
    tokio::time::resume();
    let error = result
        .expect("tool/call must have a deadline")
        .expect_err("a timeout");
    let bingo_sdk::ToolError::Failed(message) = error else {
        panic!("{error}")
    };
    assert!(
        message.contains("stub")
            && message.contains("tool/call")
            && message.contains(&bound.as_secs().to_string())
            && message.contains("callTimeout"),
        "{message}"
    );
    assert_eq!(
        cancelled(&started.manager, started.project.path()).await,
        CALL_ID
    );
    started.manager.shutdown().await;
}

async fn deadline_command(seconds: Option<u64>) {
    let started = crate::harness::started_with_timeout(&[("stub", &[])], seconds).await;
    let command = started.manager.commands().await.remove(0);
    let cx = CommandContext {
        cancel: CancellationToken::new(),
        session: SessionId::from_raw("ses_test"),
        cwd: started.project.path().into(),
        host: bingo_sdk::testing::NoHost::handle(),
    };
    tokio::time::pause();
    let mut running = Box::pin(command.run("never-answer", &cx));
    assert!(futures::poll!(&mut running).is_pending());
    let bound = Duration::from_secs(seconds.unwrap_or(600));
    tokio::time::advance(bound - Duration::from_secs(1)).await;
    assert!(futures::poll!(&mut running).is_pending());
    let result = tokio::time::timeout(Duration::from_secs(2), &mut running).await;
    drop(running);
    tokio::time::resume();
    let error = result
        .expect("command/run must have a deadline")
        .expect_err("a timeout");
    assert_eq!(error.code, bingo_sdk::ErrorCode::ToolFailed);
    assert!(
        error.message.contains("stub")
            && error.message.contains("command/run")
            && error.message.contains(&bound.as_secs().to_string())
            && error.message.contains("callTimeout"),
        "{error}"
    );
    started.manager.shutdown().await;
}

#[tokio::test]
async fn deadline_bounds_a_plugin_tool_and_abandons_it() {
    deadline_tool(None).await;
}
#[tokio::test]
async fn deadline_uses_the_manifest_for_a_plugin_tool() {
    deadline_tool(Some(5)).await;
}
#[tokio::test]
async fn deadline_bounds_a_plugin_command() {
    deadline_command(None).await;
}
#[tokio::test]
async fn deadline_uses_the_manifest_for_a_plugin_command() {
    deadline_command(Some(5)).await;
}
