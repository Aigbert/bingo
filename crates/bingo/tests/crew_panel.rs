//! The crew sheet exercises the same remote pane and command protocol as
//! other clients.
#![allow(clippy::unwrap_used, clippy::expect_used)]
use crate::support;
use bingo_sdk::*;
use serde_json::{Value, json};
use support::{LIMIT, Server, ack_for, create, ready, who};
async fn command(session: &mut Attachment, line: &str) -> Value {
    let intent = IntentId::mint();
    session.handle.submit(
        intent.clone(),
        Input::text(
            line,
            Origin::surface("tui").on(render::SurfaceName::Terminal),
        ),
    );
    serde_json::to_value(ack_for(session, &intent).await).unwrap()
}
async fn draw(host: &impl HostApi, session: &SessionId) -> View {
    host.render(
        session,
        render::RenderRequest {
            component: render::Component::Pane,
            request_id: "crew".into(),
            surface: render::SurfaceName::Terminal,
            props: json!({"owner":"bingo.mod.crew","selected":null}),
            viewport: render::Viewport {
                glyphs: Default::default(),
                place: None,
                columns: 108,
                rows: 18,
            },
        },
    )
    .await
    .unwrap()
}
async fn key(host: &impl HostApi, session: &mut Attachment, key: &str) -> Value {
    let view = draw(host, &session.session).await;
    let View::Box { layout, .. } = view else {
        panic!("crew pane: {view:?}")
    };
    let action = layout
        .on_key
        .unwrap()
        .with_value(json!({"key":key}))
        .unwrap();
    press(session, action).await
}
async fn press(session: &mut Attachment, action: Action) -> Value {
    let intent = IntentId::mint();
    session.handle.submit(
        intent.clone(),
        Input::action(
            action,
            Origin::surface("tui").on(render::SurfaceName::Terminal),
        ),
    );
    serde_json::to_value(ack_for(session, &intent).await).unwrap()
}
async fn child(
    host: &impl HostApi,
    root: &Attachment,
    cwd: &std::path::Path,
    name: &str,
) -> Attachment {
    host.open(
        SessionSelector::Create {
            spec: SessionSpec {
                cwd: cwd.into(),
                parent: Some(ParentLink {
                    session: root.session.clone(),
                    item: None,
                }),
                title: Some(name.into()),
                ..Default::default()
            },
        },
        who(),
        OpenOptions::default(),
    )
    .await
    .unwrap()
}
async fn finish(host: &bingo_surface_rpc::RemoteKernel, server: &mut Server) {
    host.shutdown().await.unwrap();
    assert!(
        tokio::time::timeout(LIMIT, server.child.wait())
            .await
            .unwrap()
            .unwrap()
            .success()
    );
}
async fn plain(root: &mut Attachment, line: &str) -> Value {
    let intent = IntentId::mint();
    root.handle.submit(
        intent.clone(),
        Input::text(line, Origin::surface("rpc").on(render::SurfaceName::Text)),
    );
    serde_json::to_value(ack_for(root, &intent).await).unwrap()
}
fn crew_pane(root: &Attachment) -> bool {
    slot::placed(&root.snapshot).iter().any(|placed| {
        placed.plugin == "bingo.mod.crew"
            && matches!(&placed.slot, slot::Slot::Pane { id, .. } if id == "crew")
    })
}
async fn working(child: &mut Attachment) {
    let intent = IntentId::mint();
    child.handle.submit(
        intent.clone(),
        Input::text("Review the cut", Origin::surface("tui")),
    );
    assert!(matches!(
        ack_for(child, &intent).await,
        IntentOutcome::TurnStarted { .. }
    ));
}
async fn room_id(host: &impl HostApi, root: &Attachment) -> SessionId {
    host.sessions(SessionFilter {
        parent: Some(root.session.clone()),
        ..Default::default()
    })
    .await
    .unwrap()
    .into_iter()
    .find(|summary| summary.title.as_deref() == Some("#design"))
    .unwrap()
    .id
}
#[tokio::test(flavor = "multi_thread")]
async fn agents_opens_the_sheet_which_walks_in_and_mentions() {
    let mut server =
        Server::spawn(r#"{"responses":[{"steps":[{"delay":{"ms":60000}},{"text":"Reviewed"}]}]}"#);
    let host = ready(&mut server).await;
    let mut root = host
        .open(create(server.cwd()), who(), OpenOptions::default())
        .await
        .unwrap();
    let empty = command(&mut root, "/agents").await;
    assert_eq!(empty["result"]["surface"]["name"], "Pane", "{empty}");
    assert!(
        draw(&host, &root.session)
            .await
            .fold()
            .contains("no teammates or rooms yet")
    );
    let mut reviewer = child(&host, &root, &server.cwd(), "reviewer").await;
    command(&mut root, "/rooms design reviewer").await;
    working(&mut reviewer).await;
    let sheet = draw(&host, &root.session).await.fold();
    assert!(
        sheet.contains("Crew") && sheet.contains("1 working"),
        "{sheet}"
    );
    assert!(
        sheet.contains("Review the cut") && sheet.contains("#design"),
        "{sheet}"
    );
    let mention = key(&host, &mut root, "tab").await;
    assert_eq!(
        mention["result"]["surface"],
        json!({"name":"Composer","args":{"text":"@reviewer "}})
    );
    assert!(!crew_pane(&root));
    assert_eq!(
        command(&mut root, "/crew").await["result"]["surface"]["name"],
        "Pane"
    );
    assert!(crew_pane(&root));
    key(&host, &mut root, "down").await;
    let room = room_id(&host, &root).await;
    let walked = key_at(&host, &mut root, room.clone(), "enter").await;
    assert_eq!(
        walked["result"]["surface"],
        json!({"name":"View","args":{"view":room}})
    );
    let text = plain(&mut root, "/agents").await;
    assert!(text["result"]["view"].is_object(), "{text}");
    assert!(text["result"].get("surface").is_none());
    finish(&host, &mut server).await;
}
/// A press as the drawing that picked `selected` carries it.
async fn key_at(
    host: &impl HostApi,
    session: &mut Attachment,
    selected: SessionId,
    key: &str,
) -> Value {
    let View::Box { layout, .. } = draw(host, &session.session).await else {
        panic!("crew sheet")
    };
    let mut action = layout.on_key.unwrap();
    action.args["selected"] = json!(selected);
    press(session, action.with_value(json!({"key":key})).unwrap()).await
}
#[tokio::test(flavor = "multi_thread")]
async fn disabled_crew_keeps_the_plain_agents_table() {
    let home = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(home.path().join(".bingo")).unwrap();
    std::fs::write(
        home.path().join(".bingo/settings.json"),
        r#"{"enabledPlugins":{"bingo.mod.crew":false}}"#,
    )
    .unwrap();
    let mut server = Server::spawn_at(home, r#"{"responses":[]}"#, &[]);
    let host = ready(&mut server).await;
    let mut root = host
        .open(create(server.cwd()), who(), OpenOptions::default())
        .await
        .unwrap();
    let catalog = host.catalog(CatalogKind::Commands).await.unwrap();
    assert!(!catalog.entries.iter().any(|e| e.id == "crew"));
    for line in ["/agents", "/room"] {
        let receipt = command(&mut root, line).await;
        assert_eq!(receipt["kind"], "applied", "{line}: {receipt}");
        assert!(
            receipt["result"].get("surface").is_none(),
            "{line}: {receipt}"
        );
    }
    finish(&host, &mut server).await;
}
