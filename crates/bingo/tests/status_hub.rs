//! Status uses the same RPC commands and render contract as an outside mod.
#![allow(clippy::unwrap_used, clippy::expect_used)]
use bingo_sdk::render::{Component, RenderRequest, SurfaceName, Viewport};
use bingo_sdk::*;
use serde_json::json;
mod support;
use support::{LIMIT, Server, ack_for, create, ready, who};

fn request(state: &SessionState) -> RenderRequest {
    RenderRequest {
        component: Component::Status,
        request_id: "status".into(),
        surface: SurfaceName::Terminal,
        props: json!(component::StatusProps::from(state)),
        viewport: Viewport {
            place: None,
            columns: 108,
            rows: 14,
        },
    }
}
async fn command(attachment: &mut Attachment, text: &str) -> IntentOutcome {
    let intent = IntentId::mint();
    attachment
        .handle
        .submit(intent.clone(), Input::text(text, Origin::surface("test")));
    ack_for(attachment, &intent).await
}
async fn key(attachment: &mut Attachment, tree: &View, key: &str) -> IntentOutcome {
    let View::Box { layout, .. } = tree else {
        panic!("status tree: {tree:?}")
    };
    let mut action = layout.on_key.clone().unwrap();
    action.args["value"] = json!({"key":key});
    let intent = IntentId::mint();
    attachment.handle.submit(
        intent.clone(),
        Input::action(action, Origin::surface("test")),
    );
    let outcome = ack_for(attachment, &intent).await;
    assert!(matches!(outcome, IntentOutcome::Applied { .. }));
    outcome
}
#[tokio::test(flavor = "multi_thread")]
async fn status_opens_updates_dispatches_and_closes_over_rpc() {
    let mut server = Server::spawn(r#"{"responses":[{"steps":[{"text":"Ready."}]}]}"#);
    let host = ready(&mut server).await;
    let mut attachment = host
        .open(create(server.cwd()), who(), OpenOptions::default())
        .await
        .unwrap();
    assert!(matches!(
        command(&mut attachment, "/status").await,
        IntentOutcome::Applied { .. }
    ));
    let id = attachment.session.clone();
    let tree = host
        .render(&id, request(&attachment.snapshot))
        .await
        .unwrap();
    assert!(tree.fold().contains("❯ session"), "{tree:?}");
    key(&mut attachment, &tree, "down").await;
    let tree = host
        .render(&id, request(&attachment.snapshot))
        .await
        .unwrap();
    assert!(tree.fold().contains("❯ model"));
    command(&mut attachment, "/rename Updated").await;
    let tree = host
        .render(&id, request(&attachment.snapshot))
        .await
        .unwrap();
    assert!(tree.fold().contains("Updated"));
    assert!(tree.fold().contains("❯ model"));
    let outcome = key(&mut attachment, &tree, "enter").await;
    assert!(serde_json::to_value(outcome).unwrap()["result"]["view"].is_object());
    command(&mut attachment, "/status").await;
    let tree = host
        .render(&id, request(&attachment.snapshot))
        .await
        .unwrap();
    let outcome = key(&mut attachment, &tree, "enter").await;
    assert!(
        serde_json::to_value(outcome)
            .unwrap()
            .to_string()
            .contains("usage: /rename <title>")
    );
    assert!(
        !slot::placed(&attachment.snapshot)
            .iter()
            .any(|p| p.plugin == "bingo.mod.status")
    );
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
async fn disabling_the_mod_retains_the_kernel_status_table() {
    let home = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(home.path().join(".bingo")).unwrap();
    std::fs::write(
        home.path().join(".bingo/settings.json"),
        r#"{"enabledPlugins":{"bingo.mod.status":false},"update":{"check":false}}"#,
    )
    .unwrap();
    let mut server = Server::spawn_at(home, r#"{"responses":[]}"#, &[]);
    let host = ready(&mut server).await;
    let mut attachment = host
        .open(create(server.cwd()), who(), OpenOptions::default())
        .await
        .unwrap();
    let outcome = command(&mut attachment, "/status").await;
    let wire = serde_json::to_value(outcome).unwrap();
    assert_eq!(wire["result"]["view"]["kind"], "keyValue", "{wire}");
    assert!(
        !slot::placed(&attachment.snapshot)
            .iter()
            .any(|p| p.plugin == "bingo.mod.status")
    );
    host.shutdown().await.unwrap();
    assert!(
        tokio::time::timeout(LIMIT, server.child.wait())
            .await
            .unwrap()
            .unwrap()
            .success()
    );
}

#[tokio::test]
async fn an_outside_mod_can_wrap_or_replace_the_status_tree() {
    for (body, expected) in [
        (
            "const tree = await next(e); return {...tree,children:[h('Text',{},'Outside'), ...tree.children]};",
            "Outside\n status",
        ),
        ("return h('Text',{},'Replacement');", "Replacement"),
    ] {
        let home = tempfile::tempdir().unwrap();
        let root = home.path().join("outside");
        std::fs::create_dir(&root).unwrap();
        std::fs::write(
            root.join("plugin.json"),
            r#"{"name":"outside","version":"1","module":"main.ts","uses":["ui"]}"#,
        )
        .unwrap();
        std::fs::write(root.join("main.ts"), format!("export function register(on) {{ on('ui.render', {{component:'Status'}}, async ($, e, next) => {{ {body} }}); }}")).unwrap();
        let host = bingo_core::Host::build(
            vec![
                Box::new(bingo_mod_status::Status::default()),
                Box::new(
                    bingo_mods::ModsPlugin::default()
                        .with_dirs(vec![root])
                        .with_project(home.path().into()),
                ),
            ],
            bingo_core::HostConfig::new(Env::rooted(home.path())),
        )
        .await
        .unwrap();
        let attachment = host
            .open(
                SessionSelector::Create {
                    spec: SessionSpec {
                        cwd: home.path().into(),
                        driver: Driver::Log,
                        ..Default::default()
                    },
                },
                who(),
                OpenOptions::default(),
            )
            .await
            .unwrap();
        let tree = host
            .render(&attachment.session, request(&attachment.snapshot))
            .await
            .unwrap();
        assert!(tree.fold().starts_with(expected), "{tree:?}");
        host.shutdown().await;
    }
}
