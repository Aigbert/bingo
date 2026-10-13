//! Polling a real process with a controlled clock and durable host state.
use super::harness::*;
use bingo_plugin_rpc::Manager;
use bingo_sdk::{Env, LinkPoint, Next};
use futures::FutureExt;
use serde_json::{Value, json};
use std::sync::Arc;
use std::time::Duration;

async fn reverse(manager: &Manager, cwd: &std::path::Path, method: &str, params: Value) -> Value {
    let tool = only_tool(manager).await;
    let (_, result) = call(&tool, json!({"host":{"method":method,"params":params}}), cwd).await;
    serde_json::from_str(&said(&result.unwrap())).unwrap()
}

async fn tick() {
    tokio::time::pause();
    tokio::time::advance(Duration::from_secs(1)).await;
    tokio::time::resume();
}

#[tokio::test]
async fn a_manifest_change_reloads_once_and_keeps_global_state() {
    let home = installed(&[("stub", &[])]);
    let project = tempfile::tempdir().unwrap();
    let root = home.path().join(".bingo/plugins/stub");
    let listener = Listening::new();
    let reloaded = Arc::new(tokio::sync::Notify::new());
    let heard = reloaded.clone();
    listener.host.on_notice(move |_, code, _| {
        let heard = heard.clone();
        async move { if code == "PLUGIN_RELOADED" { heard.notify_one(); } Ok(()) }
    });
    let manager = Manager::new(Env::rooted(home.path()), Default::default(), Default::default())
        .with_dirs(vec![root.clone()]);
    manager.start(project.path(), listener.host.handle()).await;
    let saved = reverse(&manager, project.path(), "host/state/set", json!({"scope":"global","key":"greeting","value":"hello"})).await;
    let old_tool = only_tool(&manager).await;
    let (_, result) = call(&old_tool, json!({"announce":{"method":"commands/changed","params":{"commands":[]}}}), project.path()).await;
    result.unwrap();
    assert!(manager.commands().await.is_empty());
    let path = root.join("plugin.json");
    let manifest = std::fs::read_to_string(&path).unwrap();
    std::fs::write(&path, format!("{manifest}\n")).unwrap();
    tick().await;
    tokio::time::timeout(Duration::from_secs(10), reloaded.notified()).await.unwrap();
    assert_eq!(manager.commands().await.len(), 1);
    let read = reverse(&manager, project.path(), "host/state/get", json!({"scope":"global","key":"greeting"})).await;
    assert_eq!(read, saved);
    let (_, result) = call(&old_tool, json!({}), project.path()).await;
    assert!(result.is_err(), "the previous process is stopped");
    tick().await;
    manager.shutdown().await;
    let notices: Vec<_> = listener.all().into_iter().filter(|(_, code, _)| code == "PLUGIN_RELOADED").collect();
    assert_eq!(notices.len(), 1, "{notices:?}");
    assert!(!notices[0].2.contains("restart required"));
}

#[tokio::test]
async fn reload_fails_a_held_link_and_reports_changed_services() {
    let started = started_with(&[("stub", &[])]).await;
    // Drive reload directly to pin the held phase without relying on polling order.
    let root = started.home.path().join(".bingo/plugins/stub");
    let manifest: bingo_plugin_rpc::Manifest = serde_json::from_str(&std::fs::read_to_string(root.join("plugin.json")).unwrap()).unwrap();
    let notices = Arc::new(bingo_plugin_rpc::notice::Notices::default());
    let bridge = Arc::new(bingo_plugin_rpc::Bridge::new("reload", root.clone(), manifest.entry.rooted(&root), Value::Null, None,
        bingo_plugin_rpc::Setting { env: (&Env::rooted(started.home.path())).into(), data_dir: started.home.path().into(), notices: notices.clone(), doors: bingo_plugin_rpc::Doors::new(notices.clone()), host: started.host.clone() }));
    bridge.connect().await;
    notices.drain();
    let link = bridge.interceptors().await.into_iter().find(|link| link.link.matcher().points.contains(&LinkPoint::TurnStop)).unwrap();
    let cx = hook_context();
    let next = Next::new(|()| async { panic!("the first phase holds") }.boxed());
    let mut held = Box::pin(link.link.turn_stop(&cx, (), next));
    assert!(futures::poll!(&mut held).is_pending());
    let mut changed = manifest;
    changed.entry.args.push("--no-service".into());
    bridge.reload(changed).await;
    let error = held.await.unwrap_err();
    assert!(!error.to_string().is_empty());
    let said = notices.drain();
    assert_eq!(said.len(), 1, "{said:?}");
    assert!(said[0].text.contains("restart required"));
    assert!(!bridge.tools().await.is_empty());
    bridge.stop().await;
    started.manager.shutdown().await;
}
