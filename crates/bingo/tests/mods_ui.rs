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
            place: None,
            columns: 80,
            rows: 8,
        },
    };
    let session = attachment.snapshot.summary.id.clone();
    assert_seated_viewports(&kernel, &session, &request).await;
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
    edit_and_close(&kernel, &mut attachment, &session).await;
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

async fn assert_seated_viewports(
    kernel: &bingo_surface_rpc::RemoteKernel,
    session: &SessionId,
    request: &RenderRequest,
) {
    for (place, columns, rows) in [(slot::Place::Side, 64, 43), (slot::Place::Above, 100, 9)] {
        let mut seated = request.clone();
        seated.viewport = Viewport {
            place: Some(place),
            columns,
            rows,
        };
        let view = kernel.render(session, seated).await.unwrap();
        let label = match place {
            slot::Place::Side => "side",
            _ => "above",
        };
        assert!(
            view.fold()
                .contains(&format!("Seat: {label} {columns}x{rows}")),
            "{view:?}"
        );
    }
}

async fn edit_and_close(
    kernel: &bingo_surface_rpc::RemoteKernel,
    attachment: &mut Attachment,
    session: &SessionId,
) {
    let current = kernel
        .render(
            session,
            RenderRequest {
                component: Component::Pane,
                request_id: "counter".into(),
                surface: SurfaceName::Terminal,
                props: serde_json::json!({}),
                viewport: Viewport {
                    place: None,
                    columns: 80,
                    rows: 8,
                },
            },
        )
        .await
        .unwrap();
    let View::Box { children, .. } = current else {
        panic!("counter is a Box")
    };
    let mut typed = children
        .iter()
        .find_map(|child| match child {
            View::Input { input } => input.on_input.clone(),
            _ => None,
        })
        .unwrap();
    typed.args["value"] = serde_json::json!("Ada");
    let intent = IntentId::mint();
    attachment
        .handle
        .submit(intent.clone(), Input::Action { action: typed });
    assert!(matches!(
        ack_for(attachment, &intent).await,
        IntentOutcome::Applied { .. }
    ));
    let intent = IntentId::mint();
    attachment.handle.submit(
        intent.clone(),
        Input::Action {
            action: Action {
                name: "counter:ui".into(),
                args: serde_json::json!({"handler":"counter","event":"close"}),
            },
        },
    );
    assert!(matches!(
        ack_for(attachment, &intent).await,
        IntentOutcome::Applied { .. }
    ));
    assert!(
        !slot::placed(&attachment.snapshot)
            .iter()
            .any(|placed| matches!(&placed.slot,Slot::Pane{id,..} if id=="counter"))
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn a_corner_declares_a_rect_and_fills_its_received_viewport() {
    let fixture = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/mods/corner");
    let home = tempfile::tempdir().unwrap();
    let installed = home.path().join(".bingo/plugins/corner");
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
            if kernel
                .catalog(CatalogKind::Commands)
                .await
                .unwrap()
                .entries
                .iter()
                .any(|entry| entry.id == "corner")
            {
                break;
            }
            tokio::task::yield_now().await;
        }
    })
    .await
    .unwrap();
    let intent = IntentId::mint();
    attachment.handle.submit(
        intent.clone(),
        Input::text("/corner", Origin::surface("test")),
    );
    assert!(matches!(
        ack_for(&mut attachment, &intent).await,
        IntentOutcome::Applied { .. }
    ));
    let placed = slot::placed(&attachment.snapshot);
    let (place, mut request) = placed
        .into_iter()
        .find_map(|placed| match placed.slot {
            Slot::Pane {
                place,
                view: Shown::Pull { request },
                ..
            } => Some((place[0], request)),
            _ => None,
        })
        .unwrap();
    assert!(matches!(
        place,
        slot::Place::Rect(slot::Rect {
            anchor: slot::Anchor::BottomRight,
            width: slot::Length::Percent(40),
            height: slot::Length::Percent(30),
            ..
        })
    ));
    request.viewport = Viewport {
        place: Some(place),
        columns: 48,
        rows: 9,
    };
    let view = kernel
        .render(&attachment.snapshot.summary.id, request)
        .await
        .unwrap();
    assert!(
        matches!(view,View::Raster{columns:48,rows:9,ref cells} if cells.len()==9 && cells.iter().all(|line|line.chars().count()==48))
    );
    kernel.shutdown().await.unwrap();
    assert!(
        tokio::time::timeout(LIMIT, server.child.wait())
            .await
            .unwrap()
            .unwrap()
            .success()
    );
}
