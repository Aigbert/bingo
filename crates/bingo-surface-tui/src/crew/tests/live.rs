use bingo_sdk::*;
use futures::StreamExt;
use serde_json::{Value, json};
use std::sync::Arc;
async fn submit(attached: &mut Attachment, input: Input) -> Value {
    let id = IntentId::mint();
    attached.handle.submit(id.clone(), input);
    tokio::time::timeout(std::time::Duration::from_secs(10), async {
        while let Some(frame) = attached.events.next().await {
            attached.snapshot.apply(&frame);
            if let Event::IntentAck { intent, outcome } = frame.event
                && intent == id
            {
                return serde_json::to_value(outcome).unwrap();
            }
        }
        panic!("stream ended")
    })
    .await
    .unwrap()
}
async fn line(attached: &mut Attachment, line: &str) -> Value {
    submit(
        attached,
        Input::text(
            line,
            Origin::surface("tui").on(render::SurfaceName::Terminal),
        ),
    )
    .await
}
async fn drawing(host: &bingo_core::Host, session: &SessionId) -> View {
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
fn backend(view: &View) -> String {
    use ratatui::{Terminal, backend::TestBackend, widgets::Paragraph};
    let mut terminal = Terminal::new(TestBackend::new(108, 30)).unwrap();
    terminal
        .draw(|f| f.render_widget(Paragraph::new(crate::views::render(view, 108)), f.area()))
        .unwrap();
    terminal
        .backend()
        .buffer()
        .content()
        .chunks(108)
        .map(|row| row.iter().map(|cell| cell.symbol()).collect::<String>())
        .collect::<Vec<_>>()
        .join("\n")
}
async fn key(host: &bingo_core::Host, attached: &mut Attachment, key: &str) -> Value {
    let view = drawing(host, &attached.session).await;
    assert!(backend(&view).contains("crew"));
    let View::Box { layout, .. } = view else {
        panic!("pane")
    };
    let action = layout
        .on_key
        .unwrap()
        .with_value(json!({"key":key}))
        .unwrap();
    assert_eq!(action.name, CommandSpec::ui_name("bingo.mod.crew"));
    submit(attached, crate::pressed("bingo.mod.crew", action)).await
}
async fn open(
    host: &bingo_core::Host,
    home: &std::path::Path,
    parent: Option<SessionId>,
    name: &str,
) -> Attachment {
    host.open(
        SessionSelector::Create {
            spec: SessionSpec {
                cwd: home.into(),
                parent: parent.map(|session| ParentLink {
                    session,
                    item: None,
                }),
                title: Some(name.into()),
                ..Default::default()
            },
        },
        ClientIdentity {
            name: "test".into(),
            surface: "terminal".into(),
        },
        OpenOptions::default(),
    )
    .await
    .unwrap()
}
async fn host(home: &std::path::Path) -> Arc<bingo_core::Host> {
    let script = serde_json::from_value(
        json!({"responses":[{"steps":[{"delay":{"ms":60000}},{"text":"Done"}]}]}),
    )
    .unwrap();
    bingo_core::Host::build(
        vec![
            Box::new(bingo_provider_fake::FakePlugin::new(Arc::new(
                bingo_provider_fake::FakeProvider::new(script),
            ))),
            Box::new(bingo_agents::AgentsPlugin::default()),
            Box::new(bingo_tasks::TasksPlugin::default()),
            Box::new(bingo_mod_crew::Crew::default()),
        ],
        bingo_core::HostConfig::new(Env::rooted(home)),
    )
    .await
    .unwrap()
}
#[tokio::test]
async fn backend_keys_stop_refuse_prefill_and_tick_without_rooms() {
    let home = tempfile::tempdir().unwrap();
    let host = host(home.path()).await;
    let mut root = open(&host, home.path(), None, "parent").await;
    let mut reviewer = open(&host, home.path(), Some(root.session.clone()), "reviewer").await;
    let mut scout = open(&host, home.path(), Some(root.session.clone()), "scout").await;
    assert_eq!(
        line(&mut reviewer, "Review the cut").await["kind"],
        "turnStarted"
    );
    line(&mut root, "/tasks add Check the cut").await;
    line(&mut root, "/crew").await;
    assert!(!backend(&drawing(&host, &root.session).await).contains("rooms ("));
    key(&host, &mut root, "s").await;
    confirm_stop_and_follow(&host, &mut root).await;
    let fill = key(&host, &mut root, "m").await;
    assert_eq!(
        fill["result"]["surface"]["args"]["text"],
        "/agents reopen reviewer "
    );
    let sent = line(&mut root, "/agents message reviewer Check again").await;
    assert_eq!(sent["kind"], "applied");
    assert!(sent["result"].get("surface").is_none());
    line(&mut root, "/crew").await;
    key(&host, &mut root, "]").await;
    key(&host, &mut root, " ").await;
    assert!(backend(&drawing(&host, &root.session).await).contains("☑ #1"));
    line(&mut scout, "/crew").await;
    key(&host, &mut scout, "x").await;
    assert!(backend(&drawing(&host, &scout.session).await).contains("delete its journal?"));
    let refused = key(&host, &mut scout, "enter").await;
    assert!(
        refused["result"]["message"]
            .as_str()
            .unwrap()
            .contains("beside you, not yours"),
        "{refused}"
    );
    host.shutdown().await;
}

async fn confirm_stop_and_follow(host: &bingo_core::Host, root: &mut Attachment) {
    let stopped = key(host, root, "enter").await;
    assert!(
        stopped["result"]["message"]
            .as_str()
            .unwrap()
            .contains("stopped"),
        "{stopped}"
    );
    tokio::time::timeout(std::time::Duration::from_secs(10), async {
        loop {
            if backend(&drawing(host, &root.session).await).contains("■ stopped") {
                break;
            }
            tokio::task::yield_now().await;
        }
    })
    .await
    .unwrap();
}
