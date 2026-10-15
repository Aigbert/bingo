//! Original component examples through the same stdio wire used by surfaces.
#![allow(clippy::unwrap_used, clippy::expect_used)]
mod support;
use bingo_sdk::{
    render::{Component, RenderRequest, SurfaceName, Viewport},
    *,
};
use serde_json::json;
use std::path::{Path, PathBuf};
use support::{LIMIT, Server, ack_for, create, ready, until_completed, who};

fn install(home: &Path, name: &str) -> PathBuf {
    let source = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../examples/plugins")
        .join(name);
    let target = home.join(name);
    std::fs::create_dir_all(target.join("types")).unwrap();
    for file in [
        "plugin.json",
        "register.ts",
        "types/index.d.ts",
        "README.md",
    ] {
        std::fs::copy(source.join(file), target.join(file)).unwrap();
    }
    target
}
fn settings(home: &Path, dirs: &[PathBuf]) -> PathBuf {
    let path = home.join("components.json");
    std::fs::write(&path, json!({"pluginDirs":dirs}).to_string()).unwrap();
    path
}
fn request(component: Component, id: &str, props: serde_json::Value) -> RenderRequest {
    RenderRequest {
        component,
        request_id: id.into(),
        surface: SurfaceName::Terminal,
        props,
        viewport: Viewport {
            place: None,
            columns: 80,
            rows: 8,
        },
    }
}
async fn command(attachment: &mut Attachment, text: &str) {
    let intent = IntentId::mint();
    attachment
        .handle
        .submit(intent.clone(), Input::text(text, Origin::surface("rpc")));
    let outcome = ack_for(attachment, &intent).await;
    assert!(
        if text.starts_with('/') {
            matches!(outcome, IntentOutcome::Applied { .. })
        } else {
            matches!(outcome, IntentOutcome::TurnStarted { .. })
        },
        "{text}: {outcome:?}"
    );
}
async fn stop(kernel: &bingo_surface_rpc::RemoteKernel, server: &mut Server) {
    kernel.shutdown().await.unwrap();
    assert!(
        tokio::time::timeout(LIMIT, server.child.wait())
            .await
            .unwrap()
            .unwrap()
            .success()
    );
}
fn press(view: &View) -> Option<Action> {
    match view {
        View::Button { button, .. } => button.on_press.clone(),
        View::Box { children, .. } => children.iter().find_map(press),
        _ => None,
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn band_renders_and_hides_after_its_press_over_rpc() {
    let home = tempfile::tempdir().unwrap();
    let band = install(home.path(), "band");
    let config = settings(home.path(), &[band]);
    let mut server = Server::spawn_at(
        home,
        r#"{"responses":[{"steps":[{"text":"Ready"}]}]}"#,
        &["--provider", "fake", "--settings", config.to_str().unwrap()],
    );
    let kernel = ready(&mut server).await;
    let mut attachment = kernel
        .open(create(server.cwd()), who(), OpenOptions::default())
        .await
        .unwrap();
    let session = attachment.session.clone();
    let request = request(
        Component::AbovePrompt,
        "AbovePrompt",
        json!({"hasSurvey":false}),
    );
    let first = kernel.render(&session, request.clone()).await.unwrap();
    assert!(first.fold().contains("A small band"), "{first:?}");
    let intent = IntentId::mint();
    attachment.handle.submit(
        intent.clone(),
        Input::Action {
            action: press(&first).expect("hide handler"),
            origin: Some(bingo_sdk::Origin::surface("tui")),
        },
    );
    assert!(matches!(
        ack_for(&mut attachment, &intent).await,
        IntentOutcome::Applied { .. }
    ));
    assert!(
        kernel
            .render(&session, request)
            .await
            .unwrap()
            .fold()
            .is_empty()
    );
    let survey = kernel
        .render(&session, super_request_survey())
        .await
        .unwrap();
    assert!(matches!(
        survey,
        View::Engine {
            component: Component::AbovePrompt,
            ..
        }
    ));
    stop(&kernel, &mut server).await;
}
fn super_request_survey() -> RenderRequest {
    request(
        Component::AbovePrompt,
        "AbovePrompt",
        json!({"hasSurvey":true}),
    )
}

const SCRIPT: &str = r#"{"responses":[
 {"steps":[{"toolCall":{"name":"Edit","input":{"file_path":"protected.txt","old_string":"original","new_string":"changed"}}}]},
 {"steps":[{"toolCall":{"name":"Bash","input":{"command":"echo before"}}}]},
 {"steps":[{"toolCall":{"name":"Read","input":{"file_path":"protected.txt"}}}]},
 {"steps":[{"text":"All done."}]}
]}"#;
#[tokio::test(flavor = "multi_thread")]
async fn tool_policy_rewrite_count_and_pane_rows_survive_reload() {
    let home = tempfile::tempdir().unwrap();
    let tool = install(home.path(), "tool-call");
    let pane = install(home.path(), "pane");
    std::fs::write(home.path().join("protected.txt"), "original").unwrap();
    let config = settings(home.path(), &[tool, pane.clone()]);
    let mut server = Server::spawn_at(
        home,
        SCRIPT,
        &[
            "--provider",
            "fake",
            "--allowed-tools",
            "Edit,Bash(echo:*),Read",
            "--settings",
            config.to_str().unwrap(),
        ],
    );
    let kernel = ready(&mut server).await;
    let mut attachment = kernel
        .open(create(server.cwd()), who(), OpenOptions::default())
        .await
        .unwrap();
    wait_command(&kernel, "ledger").await;
    command(&mut attachment, "Run the example calls").await;
    until_completed(&mut attachment).await;
    assert_eq!(
        std::fs::read_to_string(server.cwd().join("protected.txt")).unwrap(),
        "original"
    );
    let items = attachment.snapshot.transcript.items();
    assert!(items.iter().any(|item| matches!(&item.body, ItemBody::ToolCall {name, output:Some(output),..} if name == "Edit" && output.is_error)));
    assert!(items.iter().any(|item| matches!(&item.body, ItemBody::ToolCall {name, output:Some(output),..} if name == "Bash" && output.parts.iter().any(|part| part.as_text().is_some_and(|text| text.contains("after"))))));
    assert!(slot::placed(&attachment.snapshot).iter().any(
        |placed| matches!(&placed.slot,slot::Slot::Status{text,..} if text == "Example calls: 2")
    ));
    command(&mut attachment, "/ledger").await;
    let session = attachment.session.clone();
    let request = request(Component::Pane, "ledger", json!({}));
    let first = kernel.render(&session, request.clone()).await.unwrap();
    assert_eq!(first.fold().lines().count(), 2, "{first:?}");
    assert!(first.fold().contains("Bash") && first.fold().contains("Read"));
    let source = std::fs::read_to_string(pane.join("register.ts")).unwrap();
    std::fs::write(
        pane.join("register.ts"),
        source.replace("`${call.tool} ·", "`Reloaded ${call.tool} ·"),
    )
    .unwrap();
    command(&mut attachment, "/ledger").await;
    let reloaded = kernel.render(&session, request).await.unwrap();
    assert_eq!(reloaded.fold().lines().count(), 2);
    assert!(reloaded.fold().contains("Reloaded Bash"), "{reloaded:?}");
    stop(&kernel, &mut server).await;
}
async fn wait_command(kernel: &bingo_surface_rpc::RemoteKernel, name: &str) {
    tokio::time::timeout(LIMIT, async {
        loop {
            if kernel
                .catalog(CatalogKind::Commands)
                .await
                .unwrap()
                .entries
                .iter()
                .any(|entry| entry.id == name)
            {
                return;
            }
            tokio::task::yield_now().await;
        }
    })
    .await
    .expect("module command registers");
}

#[tokio::test(flavor = "multi_thread")]
async fn fact_sites_replace_and_wrap_over_rpc_on_each_surface() {
    let home = tempfile::tempdir().unwrap();
    let source = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/mods/sites");
    let target = home.path().join("sites");
    std::fs::create_dir(&target).unwrap();
    for file in ["plugin.json", "register.ts"] {
        std::fs::copy(source.join(file), target.join(file)).unwrap();
    }
    let config = settings(home.path(), &[target]);
    let mut server = Server::spawn_at(
        home,
        r#"{"responses":[]}"#,
        &["--provider", "fake", "--settings", config.to_str().unwrap()],
    );
    let kernel = ready(&mut server).await;
    let attachment = kernel
        .open(create(server.cwd()), who(), OpenOptions::default())
        .await
        .unwrap();
    for surface in Component::Compaction.surfaces() {
        let mut compact = request(
            Component::Compaction,
            "compacted",
            json!({"summary":"summary","replaced":3,"before":100,"after":20,"durationMs":500}),
        );
        compact.surface = *surface;
        let view = kernel.render(&attachment.session, compact).await.unwrap();
        let name = serde_json::to_value(surface).unwrap();
        assert_eq!(
            view.fold(),
            format!("Compacted 100 to 20 on {}", name.as_str().unwrap())
        );
        let mut reasoning = request(
            Component::Reasoning,
            "thought",
            json!({"text":"original","status":"completed"}),
        );
        reasoning.surface = *surface;
        let view = kernel.render(&attachment.session, reasoning).await.unwrap();
        let View::Box { children, .. } = view else {
            panic!("wrapped drawing");
        };
        assert_eq!(children[0].fold(), "Thinking follows");
        assert!(
            matches!(&children[1], View::Engine {component: Component::Reasoning, props} if props["text"] == "revised thought")
        );
    }
    stop(&kernel, &mut server).await;
}
