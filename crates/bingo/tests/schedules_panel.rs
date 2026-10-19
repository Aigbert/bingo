//! The panel's person-only lines through an isolated real process.
#![allow(clippy::unwrap_used, clippy::expect_used)]
use crate::support;
use bingo_sdk::render::{Component, RenderRequest, SurfaceName, Viewport};
use bingo_sdk::*;
use serde_json::{Value, json};
use support::{LIMIT, Server, ack_for, create, ready, who};
async fn command(a: &mut Attachment, line: &str) -> Value {
    let id = IntentId::mint();
    a.handle.submit(
        id.clone(),
        Input::text(line, Origin::surface("tui").on(SurfaceName::Terminal)),
    );
    let outcome = ack_for(a, &id).await;
    assert!(
        matches!(outcome, IntentOutcome::Applied { .. }),
        "{outcome:?}"
    );
    serde_json::to_value(outcome).unwrap()["result"].clone()
}
fn request() -> RenderRequest {
    RenderRequest {
        component: Component::Pane,
        request_id: "schedules".into(),
        surface: SurfaceName::Terminal,
        props: json!({"owner":"bingo.mod.schedule"}),
        viewport: Viewport {
            glyphs: Default::default(),
            place: None,
            columns: 108,
            rows: 20,
        },
    }
}
async fn press(host: &impl HostApi, a: &mut Attachment, key: &str) -> Value {
    press_request(host, a, key, request()).await
}
async fn press_request(
    host: &impl HostApi,
    a: &mut Attachment,
    key: &str,
    request: RenderRequest,
) -> Value {
    let tree = host.render(&a.session, request).await.unwrap();
    let View::Box { layout, .. } = tree else {
        panic!("pane")
    };
    let mut action = layout.on_key.unwrap();
    action.args["value"] = json!({"key":key});
    let id = IntentId::mint();
    a.handle
        .submit(id.clone(), Input::action(action, Origin::surface("tui")));
    let ack = serde_json::to_value(ack_for(a, &id).await).unwrap();
    assert!(matches!(ack["kind"].as_str(), Some("applied")), "{ack}");
    ack["result"].clone()
}
async fn stop(host: &bingo_surface_rpc::RemoteKernel, server: &mut Server) {
    host.shutdown().await.unwrap();
    assert!(
        tokio::time::timeout(LIMIT, server.child.wait())
            .await
            .unwrap()
            .unwrap()
            .success()
    );
}
#[tokio::test(flavor = "multi_thread")]
async fn person_lines_and_sheet_keys_share_one_store() {
    let mut server = Server::spawn(r#"{"responses":[{"steps":[{"text":"ran once"}]}]}"#);
    let host = ready(&mut server).await;
    let mut a = host
        .open(create(server.cwd()), who(), OpenOptions::default())
        .await
        .unwrap();
    command(&mut a, "/schedule every 2h check the build").await;
    confirm_model_entry(&host, &mut a, &server.cwd()).await;
    command(&mut a, "/schedule").await;
    assert!(
        host.render(&a.session, request())
            .await
            .unwrap()
            .fold()
            .contains("check the build")
    );
    let pause = press(&host, &mut a, "space").await;
    command(
        &mut a,
        pause["surface"]["args"]["value"]["run"].as_str().unwrap(),
    )
    .await;
    assert!(
        host.render(&a.session, request())
            .await
            .unwrap()
            .fold()
            .contains("paused")
    );
    let resume = press(&host, &mut a, "space").await;
    command(
        &mut a,
        resume["surface"]["args"]["value"]["run"].as_str().unwrap(),
    )
    .await;
    for key in ["r", "d", "c"] {
        assert!(press(&host, &mut a, key).await["surface"].is_null());
    }
    let run = press(&host, &mut a, "enter").await;
    assert_eq!(
        run["surface"],
        json!({"name":"SubmitAtHome","args":{"text":"check the build"}})
    );
    a.handle.submit(
        IntentId::mint(),
        Input::text("check the build", Origin::surface("tui")),
    );
    support::until_completed(&mut a).await;
    assert!(
        host.sessions(SessionFilter::default())
            .await
            .unwrap()
            .iter()
            .all(|s| s.key.as_deref().is_none_or(|k| !k.starts_with("schedule/")))
    );
    stop(&host, &mut server).await;
}

#[tokio::test(flavor = "multi_thread")]
async fn disabled_schedule_sheet_keeps_command_results_and_the_wake_sheet() {
    let home = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(home.path().join(".bingo")).unwrap();
    std::fs::write(
        home.path().join(".bingo/settings.json"),
        r#"{"enabledPlugins":{"bingo.mod.schedule":false},"update":{"check":false}}"#,
    )
    .unwrap();
    let mut server = Server::spawn_at(home, r#"{"responses":[]}"#, &[]);
    let host = ready(&mut server).await;
    let mut a = host
        .open(create(server.cwd()), who(), OpenOptions::default())
        .await
        .unwrap();
    command(&mut a, "/schedule every 2h check").await;
    assert!(command(&mut a, "/schedule").await["view"].is_object());
    let opened = command(&mut a, "/wake").await;
    assert_eq!(opened["surface"]["args"]["plugin"], "bingo.schedule");
    assert!(
        host.render(&a.session, wake_request(None))
            .await
            .unwrap()
            .fold()
            .contains("in 10s")
    );
    stop(&host, &mut server).await;
}

async fn confirm_model_entry(_host: &impl HostApi, a: &mut Attachment, home: &std::path::Path) {
    let directory = home.join(".bingo/data/schedules");
    let path = std::fs::read_dir(directory)
        .unwrap()
        .flatten()
        .map(|entry| entry.path())
        .find(|path| path.extension().is_some_and(|e| e == "json"))
        .unwrap();
    let mut entry: Value = serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
    assert_eq!(entry["by"], "person");
    assert!(entry.get("permissionMode").is_none());
    entry["by"] = json!("tool");
    std::fs::write(&path, serde_json::to_vec(&entry).unwrap()).unwrap();
    let digest =
        bingo_services::schedule::confirmation_digest(entry["text"].as_str().unwrap(), None);
    let line = format!(
        "/schedule confirm {} {digest}",
        path.file_stem().unwrap().to_str().unwrap()
    );
    reject_changed_confirmation(a, &line, &path, &mut entry).await;
    command(a, &line).await;
    let entry: Value = serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap();
    assert_eq!(entry["by"], "person");
}

async fn reject_changed_confirmation(
    a: &mut Attachment,
    line: &str,
    path: &std::path::Path,
    entry: &mut Value,
) {
    let original = entry["text"].clone();
    entry["text"] = json!("changed after Enter");
    std::fs::write(path, serde_json::to_vec(entry).unwrap()).unwrap();
    let intent = IntentId::mint();
    a.handle
        .submit(intent.clone(), Input::text(line, Origin::surface("tui")));
    let IntentOutcome::Rejected { error } = ack_for(a, &intent).await else {
        panic!("a stale confirmation must be refused");
    };
    assert!(error.message.contains("changed since it was shown"));
    let saved: Value = serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap();
    assert_eq!(saved["by"], "tool");
    entry["text"] = original;
    std::fs::write(path, serde_json::to_vec(entry).unwrap()).unwrap();
}

fn wake_request(selected: Option<&str>) -> RenderRequest {
    RenderRequest {
        request_id: "wake.sheet".into(),
        props: json!({"owner":"bingo.schedule","selected":selected}),
        ..request()
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn the_wake_sheet_cancels_only_the_wake_and_ignores_letters() {
    let mut server = Server::spawn(
        r#"{"responses":[{"steps":[{"toolCall":{"name":"Wake","input":{"after":"4m","note":"check CI"}}}]},{"steps":[{"text":"waiting"}]}]}"#,
    );
    let host = ready(&mut server).await;
    let mut a = host
        .open(create(server.cwd()), who(), OpenOptions::default())
        .await
        .unwrap();
    command(&mut a, "/schedule every 2h keep this schedule").await;
    a.handle.submit(
        IntentId::mint(),
        Input::text("set a wake", Origin::surface("tui")),
    );
    support::until_completed(&mut a).await;
    let opened = command(&mut a, "/wake").await;
    assert_eq!(opened["surface"]["args"]["plugin"], "bingo.schedule");
    let tree = host.render(&a.session, wake_request(None)).await.unwrap();
    assert!(tree.fold().contains("cancel the wake-up"));
    assert!(tree.fold().contains("in 1h"));
    press_request(&host, &mut a, "d", wake_request(Some("cancel"))).await;
    assert!(
        host.render(&a.session, wake_request(None))
            .await
            .unwrap()
            .fold()
            .contains("cancel the wake-up")
    );
    let ended = press_request(&host, &mut a, "enter", wake_request(Some("cancel"))).await;
    assert_eq!(ended["message"], "wake-up cancelled");
    assert!(
        !host
            .render(&a.session, wake_request(None))
            .await
            .unwrap()
            .fold()
            .contains("cancel the wake-up")
    );
    command(&mut a, "/schedule").await;
    assert!(
        host.render(&a.session, request())
            .await
            .unwrap()
            .fold()
            .contains("keep this schedule")
    );
    stop(&host, &mut server).await;
}
