//! The same surface wire that pulls a pane sends its controls back as actions.
#![allow(clippy::unwrap_used, clippy::expect_used)]
use bingo_sdk::{
    render::{Component, RenderRequest, SurfaceName, Viewport},
    slot::{Shown, Slot},
    *,
};
use std::path::PathBuf;
mod support;
use support::{LIMIT, Server, ack_for, create, ready, who};

#[tokio::test(flavor = "multi_thread")]
async fn a_pulled_counter_invalidates_and_renders_after_a_press() {
    let fixture = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/mods/counter");
    let home = tempfile::tempdir().unwrap();
    let installed = home.path().join(".bingo/plugins/counter");
    std::fs::create_dir_all(&installed).unwrap();
    for name in ["plugin.json", "main.ts"] {
        std::fs::copy(fixture.join(name), installed.join(name)).unwrap();
    }
    let mut server = Server::spawn_at(
        home,
        r#"{"responses":[{"steps":[{"text":"Ready."}]}]}"#,
        &["--provider", "fake"],
    );
    let kernel = ready(&mut server).await;
    let mut attachment = kernel
        .open(create(server.cwd()), who(), OpenOptions::default())
        .await
        .unwrap();
    tokio::time::timeout(LIMIT, async {
        loop {
            let commands = kernel.catalog(CatalogKind::Commands).await.unwrap();
            assert!(
                !commands
                    .entries
                    .iter()
                    .any(|entry| entry.id == "counter:ui")
            );
            if commands.entries.iter().any(|entry| entry.id == "counter") {
                break;
            }
            tokio::task::yield_now().await;
        }
    })
    .await
    .expect("counter command registers");
    let intent = IntentId::mint();
    attachment.handle.submit(
        intent.clone(),
        Input::text("/counter", Origin::surface("test")),
    );
    assert!(matches!(
        ack_for(&mut attachment, &intent).await,
        IntentOutcome::Applied { .. }
    ));
    let placed = slot::placed(&attachment.snapshot);
    assert!(placed.iter().any(|placed| matches!(&placed.slot,Slot::Pane {asked:true,view:Shown::Pull{request},..} if request.request_id=="counter")),"{placed:?}");
    let request = RenderRequest {
        component: Component::Pane,
        request_id: "counter".into(),
        surface: SurfaceName::Terminal,
        props: serde_json::json!({}),
        viewport: Viewport {
            columns: 80,
            rows: 8,
        },
    };
    let session = attachment.snapshot.summary.id.clone();
    let first = kernel.render(&session, request.clone()).await.unwrap();
    assert!(first.fold().contains("Count: 0"), "{first:?}");
    let action = button(&first);
    let intent = IntentId::mint();
    attachment.handle.submit(
        intent.clone(),
        Input::Action {
            action: action.clone(),
        },
    );
    assert!(matches!(
        ack_for(&mut attachment, &intent).await,
        IntentOutcome::Applied { .. }
    ));
    assert!(
        attachment
            .snapshot
            .invalidated
            .get("counter")
            .is_some_and(|generation| *generation > 0)
    );
    let second = kernel.render(&session, request.clone()).await.unwrap();
    assert!(second.fold().contains("Count: 1"), "{second:?}");
    let intent = IntentId::mint();
    attachment
        .handle
        .submit(intent.clone(), Input::Action { action });
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
            .contains("Count: 1")
    );
    kernel.shutdown().await.unwrap();
    let status = tokio::time::timeout(LIMIT, server.child.wait())
        .await
        .unwrap()
        .unwrap();
    assert!(status.success());
}
fn button(view: &View) -> Action {
    let View::Box { children, .. } = view else {
        panic!("counter is a Box")
    };
    children
        .iter()
        .find_map(|child| match child {
            View::Button { button, .. } => button.on_press.clone(),
            _ => None,
        })
        .unwrap()
}
