use bingo_sdk::component::{PluginsOpen, PluginsProps};
use bingo_sdk::*;
use futures::StreamExt;
use serde_json::json;
use std::sync::Arc;
struct Feature(u8);
impl Plugin for Feature {
    fn manifest(&self) -> &'static PluginManifest {
        static DRAW: PluginManifest = PluginManifest {
            id: "drawing",
            version: "1",
            uses: &[],
            requires: &[],
            provides: &[],
            config: None,
        };
        static TOOL: PluginManifest = PluginManifest {
            id: "tooling",
            version: "1",
            uses: &[],
            requires: &[],
            provides: &[],
            config: None,
        };
        static STORE: PluginManifest = PluginManifest {
            id: "needed",
            version: "1",
            uses: &[],
            requires: &[],
            provides: &["store:fixture"],
            config: None,
        };
        match self.0 {
            0 => &DRAW,
            1 => &TOOL,
            _ => &STORE,
        }
    }
    fn register(&self, r: &mut Registrar) -> Result<(), PluginError> {
        if self.0 == 1 {
            r.add(Contribution::Tools(Arc::new(Empty)));
        }
        Ok(())
    }
}
struct Empty;
#[async_trait::async_trait]
impl ToolSource for Empty {
    fn id(&self) -> &str {
        "empty"
    }
    async fn tools(&self) -> Vec<Arc<dyn Tool>> {
        vec![]
    }
}
async fn submit(attachment: &mut Attachment, input: Input) -> IntentOutcome {
    let intent = IntentId::mint();
    attachment.handle.submit(intent.clone(), input);
    tokio::time::timeout(std::time::Duration::from_secs(10), async {
        while let Some(frame) = attachment.events.next().await {
            if let Event::IntentAck {
                intent: ack,
                outcome,
            } = frame.event
                && ack == intent
            {
                return outcome;
            }
        }
        panic!("stream ended")
    })
    .await
    .unwrap()
}
fn backend(view: View) -> String {
    use ratatui::{Terminal, backend::TestBackend, widgets::Paragraph};
    let mut terminal = Terminal::new(TestBackend::new(108, 24)).unwrap();
    terminal
        .draw(|f| f.render_widget(Paragraph::new(crate::views::render(&view, 108)), f.area()))
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
async fn panel(host: &bingo_core::Host, attachment: &Attachment) -> View {
    let props = PluginsProps {
        open: PluginsOpen::Plugins,
        session: attachment.session.clone(),
        plugins: host.catalog(CatalogKind::Plugins).await.unwrap().entries,
        action: Action {
            name: "surface:plugins".into(),
            args: json!({"handler":"panel"}),
        },
    };
    host.render(
        &attachment.session,
        render::RenderRequest {
            component: render::Component::Plugins,
            request_id: "panel".into(),
            surface: render::SurfaceName::Terminal,
            props: serde_json::to_value(props).unwrap(),
            viewport: render::Viewport {
                place: None,
                columns: 108,
                rows: 19,
            },
        },
    )
    .await
    .unwrap()
}
async fn key(host: &bingo_core::Host, attachment: &mut Attachment, key: &str) -> serde_json::Value {
    let view = panel(host, attachment).await;
    assert!(backend(view.clone()).contains("one list"));
    let View::Box { layout, .. } = view else {
        panic!("panel")
    };
    let mut action = layout.on_key.unwrap();
    action.args["value"] = json!({"key":key});
    let IntentOutcome::Applied { result } = submit(attachment, Input::Action { action }).await
    else {
        panic!("handled")
    };
    result["surface"]["args"]["value"].clone()
}
#[tokio::test]
async fn panel_keys_drive_live_switches_pending_tools_and_needed_refusal() {
    let home = tempfile::tempdir().unwrap();
    let host = bingo_core::Host::build(
        vec![
            Box::new(Feature(0)),
            Box::new(Feature(1)),
            Box::new(Feature(2)),
            Box::new(bingo_mod_plugins::Plugins),
        ],
        bingo_core::HostConfig::new(Env::rooted(home.path())),
    )
    .await
    .unwrap();
    let mut attachment = host
        .open(
            SessionSelector::Create {
                spec: SessionSpec {
                    cwd: home.path().into(),
                    driver: Driver::Log,
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
        .unwrap();
    key(&host, &mut attachment, "enter").await;
    for (name, expected) in [("drawing", "off now"), ("tooling", "off at next start")] {
        let operation = key(&host, &mut attachment, "space").await;
        assert_eq!(operation["run"], format!("/plugins disable {name}"));
        let result = submit(
            &mut attachment,
            Input::text(operation["run"].as_str().unwrap(), Origin::surface("test")),
        )
        .await;
        assert!(
            matches!(result,IntentOutcome::Applied{ref result} if result["message"].as_str().unwrap().contains(expected)),
            "{result:?}"
        );
        assert!(
            backend(panel(&host, &attachment).await).contains(if name == "drawing" {
                "○ off"
            } else {
                "off at next start"
            })
        );
        key(&host, &mut attachment, "down").await;
    }
    let refused = key(&host, &mut attachment, "space").await;
    assert!(refused["message"].as_str().unwrap().contains("stays on"));
    assert!(refused.get("run").is_none());
    host.shutdown().await;
}

struct Facts(Arc<std::sync::Mutex<bingo_services::mcp::ServerRow>>);
impl Plugin for Facts {
    fn manifest(&self) -> &'static PluginManifest {
        static M: PluginManifest = PluginManifest {
            id: "facts",
            version: "1",
            uses: &[],
            requires: &[],
            provides: &["service:mcp.servers"],
            config: None,
        };
        &M
    }
    fn register(&self, r: &mut Registrar) -> Result<(), PluginError> {
        r.add(Contribution::service(
            &bingo_services::mcp::KEY,
            Arc::new(Facts(self.0.clone())) as Arc<dyn bingo_services::mcp::Servers>,
            None,
        ));
        r.add(Contribution::Plugins(Arc::new(Facts(self.0.clone()))));
        Ok(())
    }
}
#[async_trait::async_trait]
impl bingo_services::mcp::Servers for Facts {
    async fn rows(&self) -> Vec<bingo_services::mcp::ServerRow> {
        vec![self.0.lock().unwrap().clone()]
    }
}
#[async_trait::async_trait]
impl PluginSource for Facts {
    fn id(&self) -> &str {
        "facts"
    }
    async fn plugins(&self) -> Vec<PluginStatus> {
        vec![PluginStatus {
            id: "plugins:demo".into(),
            version: "1".into(),
            state: PluginState::Waiting { changed: false },
            from: "fixture".into(),
            execution: None,
        }]
    }
}
async fn facts_scene(
    home: &std::path::Path,
    row: Arc<std::sync::Mutex<bingo_services::mcp::ServerRow>>,
) -> (Arc<bingo_core::Host>, Attachment) {
    let host = bingo_core::Host::build(
        vec![
            Box::new(Facts(row.clone())),
            Box::new(bingo_mod_plugins::Plugins),
        ],
        bingo_core::HostConfig::new(Env::rooted(home)),
    )
    .await
    .unwrap();
    let attachment = host
        .open(
            SessionSelector::Create {
                spec: SessionSpec {
                    cwd: home.into(),
                    driver: Driver::Log,
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
        .unwrap();
    (host, attachment)
}
#[tokio::test]
async fn painted_keys_confirm_full_twice_and_dispatch_each_server_state() {
    use bingo_services::mcp::Status;
    let home = tempfile::tempdir().unwrap();
    let row = Arc::new(std::sync::Mutex::new(
        serde_json::from_value(json!({
            "name":"local", "transport":{"type":"stdio","command":"fixture"},
            "status":{"kind":"disabled"}
        }))
        .unwrap(),
    ));
    let (host, mut attachment) = facts_scene(home.path(), row.clone()).await;
    key(&host, &mut attachment, "enter").await;
    key(&host, &mut attachment, "down").await;
    key(&host, &mut attachment, "down").await;
    assert!(
        key(&host, &mut attachment, "enter")
            .await
            .get("run")
            .is_none()
    );
    assert!(
        backend(panel(&host, &attachment).await).contains("again: demo could run tools unasked")
    );
    assert_eq!(
        key(&host, &mut attachment, "enter").await["run"],
        "/plugins approve plugins:demo full"
    );
    key(&host, &mut attachment, "down").await;
    key(&host, &mut attachment, "down").await;
    for (status, verb) in [
        (Status::Connected { tools: 1 }, "tools"),
        (Status::NeedsAuth { why: "401".into() }, "login"),
        (
            Status::Failed {
                why: "fixture failed".into(),
            },
            "reconnect",
        ),
        (Status::Disabled, "enable"),
        (Status::Waiting { changed: false }, "approve"),
    ] {
        let waiting = matches!(status, Status::Waiting { .. });
        row.lock().unwrap().status = status;
        if waiting {
            for key_name in ["space", "r", "enter"] {
                let reviewed = key(&host, &mut attachment, key_name).await;
                assert!(reviewed.get("run").is_none(), "{key_name}: {reviewed}");
            }
            assert!(backend(panel(&host, &attachment).await).contains("⏎ again: approve local"));
        }
        assert_eq!(
            key(&host, &mut attachment, "enter").await["run"],
            format!("/mcp {verb} local")
        );
        if matches!(verb, "tools" | "login") {
            key(&host, &mut attachment, "down").await;
            key(&host, &mut attachment, "down").await;
        }
    }
    row.lock().unwrap().status = Status::SigningIn;
    let refused = key(&host, &mut attachment, "r").await;
    assert!(refused["message"].as_str().unwrap().contains("wait"));
    assert!(refused.get("run").is_none());
    host.shutdown().await;
}
/// Presses a key on a drawing already on screen, without drawing again
/// first: what a person sees when keys outrun the redraw.
async fn key_on(view: &View, attachment: &mut Attachment, key: &str) -> serde_json::Value {
    let View::Box { layout, .. } = view else {
        panic!("panel")
    };
    let mut action = layout.on_key.clone().unwrap();
    action.args["value"] = json!({"key":key});
    let IntentOutcome::Applied { result } = submit(attachment, Input::Action { action }).await
    else {
        panic!("handled")
    };
    result["surface"]["args"]["value"].clone()
}
#[tokio::test]
async fn a_key_outrunning_the_redraw_acts_on_nothing() {
    let home = tempfile::tempdir().unwrap();
    let row = Arc::new(std::sync::Mutex::new(
        serde_json::from_value(json!({
            "name":"local", "transport":{"type":"stdio","command":"fixture"},
            "status":{"kind":"disabled"}
        }))
        .unwrap(),
    ));
    let (host, mut attachment) = facts_scene(home.path(), row).await;
    key(&host, &mut attachment, "down").await;
    let seen = panel(&host, &attachment).await;
    assert!(backend(seen.clone()).contains("❯  ▸ 2 built in"));
    key(&host, &mut attachment, "down").await;
    let stale = key_on(&seen, &mut attachment, "enter").await;
    assert!(stale.get("run").is_none(), "{stale}");
    assert!(stale["message"].as_str().unwrap().contains("changed"));
    assert_eq!(
        key(&host, &mut attachment, "enter").await["run"],
        "/mcp enable local"
    );
    host.shutdown().await;
}
