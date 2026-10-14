//! Offline choice transitions, persistence and unchanged request prefixes.
use super::commands::{Client, message};
use super::*;

static COMPACTOR: PluginManifest = PluginManifest {
    id: "test.compactor",
    version: "0",
    uses: &[],
    provides: &["compactor"],
    requires: &[],
    config: None,
};

struct CutAtTail(Arc<ScriptedCompactor>);

#[async_trait]
impl Compactor for CutAtTail {
    async fn compact(
        &self,
        cx: CompactContext<'_>,
        reason: CompactReason,
    ) -> Result<Compaction, CompactError> {
        let boundary = cx.items.last().unwrap().id.clone();
        let mut cut = self.0.compact(cx, reason).await?;
        cut.boundary = boundary;
        cut.summary = "x ".repeat(cut.after as usize * 2);
        Ok(cut)
    }
}

async fn setup(
    home: &std::path::Path,
    scripts: Vec<Script>,
    compactor: Option<Arc<ScriptedCompactor>>,
) -> (Arc<Host>, Arc<ScriptedProvider>) {
    let provider = ScriptedProvider::new(scripts);
    provider.serves(&["large", "small"]);
    let mut plugins = vec![TestPlugin::boxed(
        &PROVIDER,
        vec![Contribution::Provider(provider.clone())],
    )];
    if let Some(compactor) = compactor {
        plugins.push(TestPlugin::boxed(
            &COMPACTOR,
            vec![Contribution::Compactor(Arc::new(CutAtTail(compactor)))],
        ));
    }
    let config = HostConfig::new(env_in(home)).with_layer("test", json!({
        "provider":"scripted", "model":"large", "maxTokens":1000,
        "models":{"scripted/large":{"contextWindow":200000}, "scripted/small":{"contextWindow":8000,"images":false}}
    }));
    (Host::build(plugins, config).await.unwrap(), provider)
}

fn response(used: u64) -> Script {
    let mut events = text("hello");
    if let Ok(ModelEvent::Finish { usage, .. }) = events.last_mut().unwrap() {
        usage.input_tokens = used;
    }
    Script::Events(events)
}

#[tokio::test]
async fn a_smaller_window_compacts_on_the_old_model_before_switching() {
    let home = tempfile::tempdir().unwrap();
    let compactor = ScriptedCompactor::new(vec![ScriptedCompactor::cut("itm_none", 9000, 100)]);
    let (host, _) = setup(home.path(), vec![response(9000)], Some(compactor.clone())).await;
    let mut client = Client::open(&host).await;
    client.ack("hello").await;
    client.until_turn_completed().await;
    let (ack, frames) = client.ack("/model small").await;
    assert!(message(&ack).contains("compacted 9000 → "));
    assert!(client.state.context.unwrap().used < 6300);
    assert_eq!(compactor.requests.lock().unwrap()[0].model, "large");
    assert_eq!(
        host.session_model(&client.state.summary.id)
            .await
            .unwrap()
            .unwrap()
            .id,
        "small"
    );
    let compacted = frames
        .iter()
        .position(|frame| matches!(frame.event, Event::Compacted { .. }))
        .unwrap();
    let changed = frames.iter().position(|frame| matches!(&frame.event, Event::SessionUpdated { summary } if summary.model.as_deref() == Some("small"))).unwrap();
    assert!(compacted < changed);
    host.shutdown().await;
}

#[tokio::test]
async fn unsuccessful_compaction_preserves_the_choice_and_settings() {
    for answer in [
        Err(KernelError::new(ErrorCode::ProviderUnavailable, "offline")),
        ScriptedCompactor::cut("itm_none", 9000, 8000),
    ] {
        let home = tempfile::tempdir().unwrap();
        let compactor = ScriptedCompactor::new(vec![answer]);
        let (host, _) = setup(home.path(), vec![response(9000)], Some(compactor)).await;
        let mut client = Client::open(&host).await;
        client.ack("hello").await;
        client.until_turn_completed().await;
        let (ack, _) = client.ack("/model small --keep").await;
        assert!(matches!(ack, IntentOutcome::Rejected { .. }), "{ack:?}");
        assert_eq!(
            host.session_model(&client.state.summary.id)
                .await
                .unwrap()
                .unwrap()
                .id,
            "large"
        );
        assert!(!crate::settings::user_path(host.env()).exists());
        host.shutdown().await;
    }
}

#[tokio::test]
async fn pins_round_trip_in_order_without_touching_comments() {
    let home = tempfile::tempdir().unwrap();
    let path = crate::settings::user_path(&env_in(home.path()));
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(
        &path,
        "# Keep this comment\nmodel = \"large\" # chosen before\n",
    )
    .unwrap();
    let (host, _) = setup(home.path(), vec![], None).await;
    let mut client = Client::open(&host).await;
    let mut gateway = host.gateway_events();
    let (ack, _) = client.ack("/model pin scripted/large").await;
    assert_eq!(message(&ack), "pinned scripted/large");
    assert!(matches!(
        gateway.next().await,
        Some(GatewayEvent::CatalogChanged {
            kind: CatalogKind::Models
        })
    ));
    client.ack("/model pin scripted/small").await;
    client.ack("/model pin scripted/large").await;
    assert_eq!(
        crate::host::pins::pinned_models(&host),
        ["scripted/large", "scripted/small"]
    );
    let catalog = host.catalog(CatalogKind::Models).await.unwrap();
    assert_eq!(catalog.entries[0].id, "scripted/large");
    assert_eq!(catalog.entries[1].id, "scripted/small");
    assert_eq!(
        catalog
            .entries
            .iter()
            .find(|entry| entry.id == "scripted/large")
            .unwrap()
            .meta["pinned"],
        true
    );
    client.ack("/model unpin scripted/large").await;
    assert_eq!(crate::host::pins::pinned_models(&host), ["scripted/small"]);
    let saved = std::fs::read_to_string(&path).unwrap();
    assert!(saved.contains("# Keep this comment") && saved.contains("# chosen before"));
    let layer = crate::settings::read_layer(&path);
    assert!(layer.is_ok());
    assert_eq!(
        crate::settings::read_document(&path).unwrap()["pinned"],
        json!(["scripted/small"])
    );
    let (ack, _) = client.ack("/model pin invented").await;
    assert!(
        matches!(ack, IntentOutcome::Rejected { error } if error.message.contains("closest ids:"))
    );
    host.shutdown().await;
}

#[tokio::test]
async fn a_picture_warning_keeps_the_picture_and_the_small_switch_is_instant() {
    let home = tempfile::tempdir().unwrap();
    let (host, _) = setup(home.path(), vec![response(10)], None).await;
    let mut client = Client::open(&host).await;
    client.handle.submit(
        IntentId::mint(),
        Input::Text {
            text: "look".into(),
            delivery: Delivery::Wake,
            images: vec![Image {
                media_type: "image/png".into(),
                data: "AA==".into(),
                width: None,
                height: None,
                path: None,
            }],
            origin: Origin::surface("test"),
        },
    );
    client.until_turn_completed().await;
    let (ack, frames) = client.ack("/model small").await;
    assert!(message(&ack).contains("pictures stay, small won't see them"));
    assert!(
        !frames
            .iter()
            .any(|frame| matches!(frame.event, Event::TurnStarted { .. }))
    );
    assert!(super::super::model_switch::has_pictures(&client.state));
    host.shutdown().await;
}

#[tokio::test]
async fn a_switch_that_needs_compaction_queues_but_an_ordinary_choice_does_not() {
    tokio::time::timeout(std::time::Duration::from_secs(30), queued_switch())
        .await
        .unwrap();
}

async fn queued_switch() {
    let home = tempfile::tempdir().unwrap();
    let compactor = ScriptedCompactor::new(vec![ScriptedCompactor::cut("itm_none", 20000, 100)]);
    let (host, _) = setup(
        home.path(),
        vec![Script::Hang(vec![ModelEvent::TextStart { id: "b".into() }])],
        Some(compactor.clone()),
    )
    .await;
    let mut client = Client::open(&host).await;
    client.handle.submit(
        IntentId::mint(),
        Input::text("x ".repeat(40000), Origin::surface("test")),
    );
    while let Some(frame) = client.events.next().await {
        client.state.apply(&frame);
        if matches!(
            frame.event,
            Event::ItemCompleted {
                item: Item {
                    body: ItemBody::User { .. },
                    ..
                }
            }
        ) {
            break;
        }
    }
    let (ack, _) = client.ack("/model large").await;
    assert!(message(&ack).contains("this session"));
    let (ack, _) = client.ack("/model small").await;
    assert!(matches!(ack, IntentOutcome::Queued { .. }), "{ack:?}");
    assert!(compactor.calls.lock().unwrap().is_empty());
    client
        .handle
        .interrupt(IntentId::mint(), InterruptScope::Head);
    while let Some(frame) = client.events.next().await {
        client.state.apply(&frame);
        if let Event::IntentAck {
            outcome: IntentOutcome::Applied { result },
            ..
        } = &frame.event
            && result["message"]
                .as_str()
                .is_some_and(|text| text.contains("compacted"))
        {
            break;
        }
    }
    assert_eq!(client.state.summary.model.as_deref(), Some("small"));
    assert_eq!(compactor.requests.lock().unwrap()[0].model, "large");
    host.shutdown().await;
}

#[tokio::test]
async fn catalogue_pins_and_the_same_choice_preserve_request_prefixes() {
    let home = tempfile::tempdir().unwrap();
    let (host, provider) = setup(home.path(), vec![response(10), response(10)], None).await;
    let mut client = Client::open(&host).await;
    client.ack("hello").await;
    client.until_turn_completed().await;
    host.catalog(CatalogKind::Models).await.unwrap();
    client.ack("/model pin scripted/large").await;
    client.ack("/model large").await;
    client.ack("again").await;
    client.until_turn_completed().await;
    let requests = provider.requests();
    assert_eq!(requests[0].system, requests[1].system);
    assert_eq!(requests[0].tools, requests[1].tools);
    assert_eq!(requests[0].provider_options, requests[1].provider_options);
    assert_eq!(requests[0].reasoning, requests[1].reasoning);
    assert_eq!(requests[0].session, requests[1].session);
    assert!(requests[1].messages.starts_with(&requests[0].messages));
    host.shutdown().await;
}

#[tokio::test]
async fn a_small_summary_cannot_hide_a_retained_tail_that_still_overflows() {
    let home = tempfile::tempdir().unwrap();
    let compactor = ScriptedCompactor::new(vec![ScriptedCompactor::cut("itm_none", 9000, 100)]);
    let (host, _) = setup(
        home.path(),
        vec![Script::Events(text(&"x".repeat(40000)))],
        Some(compactor),
    )
    .await;
    let mut client = Client::open(&host).await;
    client.ack("hello").await;
    client.until_turn_completed().await;
    let (ack, _) = client.ack("/model small").await;
    assert!(
        matches!(ack, IntentOutcome::Rejected { error } if error.message.contains("context must be below"))
    );
    assert_eq!(client.state.summary.model.as_deref(), Some("large"));
    assert!(client.state.context.unwrap().used > 6300);
    host.shutdown().await;
}
