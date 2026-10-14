//! Sealed render contracts over the same stdio wire used by remote surfaces.
#![allow(clippy::unwrap_used, clippy::expect_used)]
mod support;
use bingo_sdk::{render::*, *};
use serde_json::json;
use support::{LIMIT, Server, create, ready, who};

#[tokio::test(flavor = "multi_thread")]
async fn fixture_mods_preserve_the_sealed_contract_on_every_surface() {
    for fixture in ["frame", "rewrite", "omit", "duplicate", "clip"] {
        let home = tempfile::tempdir().unwrap();
        let source = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/mods")
            .join(format!("sealed-{fixture}"));
        let installed = home
            .path()
            .join(".bingo/plugins")
            .join(format!("sealed-{fixture}"));
        std::fs::create_dir_all(&installed).unwrap();
        for file in ["plugin.json", "main.ts"] {
            std::fs::copy(source.join(file), installed.join(file)).unwrap();
        }
        let mut server = Server::spawn_at(
            home,
            r#"{"responses":[{"steps":[{"text":"Ready"}]}]}"#,
            &["--provider", "fake"],
        );
        let kernel = ready(&mut server).await;
        let attachment = kernel
            .open(create(server.cwd()), who(), OpenOptions::default())
            .await
            .unwrap();
        for surface in Component::Permission.surfaces() {
            let request = RenderRequest {
                component: Component::Permission,
                request_id: "ask".into(),
                surface: *surface,
                props: component::Props::Permission(component::PermissionProps {
                    interaction_id: InteractionId::from_raw("ask"),
                    tool: "Write".into(),
                    input: json!({"file_path":"note.txt","content":"all of it"}),
                    answers: vec![AnswerSpec::AllowOnce, AnswerSpec::Deny],
                })
                .value(),
                viewport: Viewport {
                    place: None,
                    columns: 80,
                    rows: 24,
                },
            };
            let answer = kernel
                .render(&attachment.session, request.clone())
                .await
                .unwrap();
            view::sealed_ok(request.component, &request.props, &answer).unwrap();
            if matches!(fixture, "rewrite" | "omit" | "duplicate") {
                assert_eq!(answer, request.engine(), "{fixture}: {surface:?}");
            } else {
                assert!(matches!(answer, View::Box { .. }));
            }
        }
        kernel.shutdown().await.unwrap();
        assert!(
            tokio::time::timeout(LIMIT, server.child.wait())
                .await
                .unwrap()
                .unwrap()
                .success()
        );
    }
}
