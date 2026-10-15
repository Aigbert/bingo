//! Crew exercises the same remote pane and command protocol as other clients.
#![allow(clippy::unwrap_used, clippy::expect_used)]
mod support;
use bingo_sdk::*;
use serde_json::{Value, json};
use support::{LIMIT, Server, ack_for, create, ready, who};
async fn command(session: &mut Attachment, line: &str) -> Value {
    let intent = IntentId::mint();
    session.handle.submit(
        intent.clone(),
        Input::text(
            line,
            Origin::surface("test").on(render::SurfaceName::Terminal),
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
            props: json!({}),
            viewport: render::Viewport {
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
    let mut action = layout.on_key.unwrap();
    action.args["value"] = json!({"key":key});
    let intent = IntentId::mint();
    session.handle.submit(
        intent.clone(),
        Input::action(action, Origin::surface("test")),
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
async fn has_session(host: &impl HostApi, id: &SessionId) -> bool {
    host.sessions(SessionFilter::default())
        .await
        .unwrap()
        .iter()
        .any(|summary| &summary.id == id)
}
async fn confirmed_dismissal(host: &impl HostApi, root: &mut Attachment, child: &SessionId) {
    command(root, "/crew").await;
    key(host, root, "x").await;
    assert!(
        draw(host, &root.session)
            .await
            .fold()
            .contains("delete its journal?")
    );
    assert!(has_session(host, child).await);
    key(host, root, "esc").await;
    assert!(has_session(host, child).await);
    assert!(
        !draw(host, &root.session)
            .await
            .fold()
            .contains("delete its journal?")
    );
    key(host, root, "x").await;
    key(host, root, "enter").await;
    assert!(!has_session(host, child).await);
    assert!(
        draw(host, &root.session)
            .await
            .fold()
            .contains("No agents here yet.")
    );
}
#[tokio::test(flavor = "multi_thread")]
async fn crew_stops_prefills_ticks_and_closes_through_minted_keys() {
    let mut server =
        Server::spawn(r#"{"responses":[{"steps":[{"delay":{"ms":60000}},{"text":"Reviewed"}]}]}"#);
    let host = ready(&mut server).await;
    let mut root = host
        .open(create(server.cwd()), who(), OpenOptions::default())
        .await
        .unwrap();
    let mut child = child(&host, &root, &server.cwd(), "reviewer").await;
    command(&mut root, "/tasks add Review the cut @reviewer").await;
    command(&mut root, "/rooms design reviewer").await;
    let intent = IntentId::mint();
    child.handle.submit(
        intent.clone(),
        Input::text("Review the cut", Origin::surface("test")),
    );
    assert!(matches!(
        ack_for(&mut child, &intent).await,
        IntentOutcome::TurnStarted { .. }
    ));
    let opened = command(&mut root, "/crew").await;
    assert_eq!(opened["result"]["surface"]["name"], "Pane");
    assert!(draw(&host, &root.session).await.fold().contains("reviewer"));
    let stopped = key(&host, &mut root, "s").await;
    assert!(
        stopped["result"]["message"]
            .as_str()
            .unwrap()
            .contains("stopped"),
        "{stopped}"
    );
    tokio::time::timeout(LIMIT, async {
        loop {
            if draw(&host, &root.session)
                .await
                .fold()
                .contains("■ stopped")
            {
                break;
            }
            tokio::task::yield_now().await;
        }
    })
    .await
    .unwrap();
    let fill = key(&host, &mut root, "m").await;
    assert_eq!(
        fill["result"]["surface"],
        json!({"name":"Composer","args":{"text":"/agents reopen reviewer "}})
    );
    assert!(fill["result"].get("view").is_none());
    command(&mut root, "/crew").await;
    key(&host, &mut root, "right").await;
    let picker = key(&host, &mut root, "enter").await;
    assert_eq!(picker["result"]["surface"]["name"], "Pane");
    assert_eq!(
        picker["result"]["surface"]["args"]["plugin"],
        "bingo.mod.models"
    );
    assert_eq!(
        picker["result"]["surface"]["args"]["view"],
        json!(child.session)
    );
    command(&mut root, "/crew").await;
    key(&host, &mut root, "]").await;
    assert!(draw(&host, &root.session).await.fold().contains("#design"));
    key(&host, &mut root, "]").await;
    key(&host, &mut root, " ").await;
    assert!(draw(&host, &root.session).await.fold().contains("☑ #1"));
    key(&host, &mut root, "esc").await;
    confirmed_dismissal(&host, &mut root, &child.session).await;
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
async fn disabled_crew_retains_tables_and_no_key() {
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
    for line in ["/agents", "/rooms", "/tasks"] {
        assert_eq!(command(&mut root, line).await["kind"], "applied");
    }
    host.shutdown().await.unwrap();
    assert!(
        tokio::time::timeout(LIMIT, server.child.wait())
            .await
            .unwrap()
            .unwrap()
            .success()
    );
}
