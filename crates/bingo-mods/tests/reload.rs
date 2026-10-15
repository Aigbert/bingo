mod support;
use bingo_sdk::*;
use futures::StreamExt;
use serde_json::{Value, json};
use std::{sync::Arc, time::Duration};
use support::Fixture;
const SOURCE: &str = r#"
export function register(on) {
  let starts = 0;
  on('session.start', () => { starts++; });
  on('fixture.echo', async ($, e) => {
    if (e.save) await $.store.set('saved', e.save);
    return {generation:'old', saved:await $.store.get('saved'), starts};
  });
  on('tool.check', async ($, e, next) => { await $.gate.wait({}); return next(e); });
}
"#;

#[tokio::test]
async fn reload_keeps_store_and_restarts_every_open_session() {
    let fixture = Fixture::new(SOURCE, &["store"], json!({})).await;
    let second = fixture
        .host
        .open(
            SessionSelector::Create {
                spec: SessionSpec {
                    driver: Driver::Log,
                    cwd: fixture.home.path().into(),
                    ..Default::default()
                },
            },
            ClientIdentity {
                name: "second".into(),
                surface: "test".into(),
            },
            OpenOptions::default(),
        )
        .await
        .unwrap();
    fixture
        .call("fixture", "echo", json!({"save":"kept"}))
        .await;
    std::fs::write(
        fixture.root.join("main.ts"),
        SOURCE.replace("'old'", "'new'"),
    )
    .unwrap();
    fixture.plugin.reload(&fixture.root).await.unwrap();
    let result = fixture.call("fixture", "echo", json!({})).await;
    assert_eq!(
        result,
        json!({"generation":"new", "saved":{"version":1,"value":"kept"},"starts":2})
    );
    drop(second);
    fixture.host.shutdown().await;
}

struct Gate {
    entered: tokio::sync::Notify,
    release: tokio::sync::Notify,
}
#[async_trait::async_trait]
impl WireService for Gate {
    fn face(&self) -> Option<Face> {
        Some(Face::new("gate").method::<Value, Value>("wait", false))
    }
    async fn call(&self, _: &str, _: Value) -> Result<Value, ServiceError> {
        self.entered.notify_one();
        self.release.notified().await;
        Ok(Value::Null)
    }
}
#[tokio::test]
async fn a_mid_turn_edit_waits_for_turn_complete() {
    let mut fixture = Fixture::agent(SOURCE, &["store", "gate"], json!({})).await;
    let gate = Arc::new(Gate {
        entered: Default::default(),
        release: Default::default(),
    });
    fixture.host.open_service("gate", gate.clone()).unwrap();
    fixture
        .attachment
        .handle
        .submit(IntentId::mint(), Input::text("go", Origin::surface("test")));
    tokio::time::timeout(Duration::from_secs(5), gate.entered.notified())
        .await
        .unwrap();
    fixture
        .call("fixture", "echo", json!({"save":"kept"}))
        .await;
    std::fs::write(
        fixture.root.join("main.ts"),
        SOURCE.replace("'old'", "'changed'"),
    )
    .unwrap();
    tokio::time::pause();
    tokio::time::advance(Duration::from_secs(1)).await;
    tokio::time::resume();
    tokio::time::timeout(Duration::from_secs(5), async {
        while let Some(frame) = fixture.attachment.events.next().await {
            if let Event::ItemCompleted { item } = frame.event
                && matches!(item.body, ItemBody::Notice {code, ..} if code == "PLUGIN_STALE")
            {
                break;
            }
        }
    })
    .await
    .expect("explicit-directory polling marks the module stale");
    assert_eq!(
        fixture.call("fixture", "echo", json!({})).await["generation"],
        "old"
    );
    gate.release.notify_one();
    tokio::time::timeout(Duration::from_secs(5), async {
        while let Some(frame) = fixture.attachment.events.next().await {
            if let Event::TurnCompleted { status, .. } = frame.event {
                assert_eq!(status, TurnStatus::Completed);
                break;
            }
        }
    })
    .await
    .unwrap();
    let result = fixture.call("fixture", "echo", json!({})).await;
    assert_eq!(result["generation"], "changed");
    assert_eq!(result["saved"]["value"], "kept");
    fixture.host.shutdown().await;
}

#[tokio::test]
async fn disabling_during_a_turn_preserves_its_links_through_completion() {
    let source = SOURCE.replace("  on('tool.check'", "  on('turn.complete', async ($, e, next) => { await $.store.set('completed', true); return next(e); });\n  on('tool.check'");
    let mut fixture = Fixture::agent(&source, &["store", "gate"], json!({})).await;
    let gate = Arc::new(Gate {
        entered: Default::default(),
        release: Default::default(),
    });
    fixture.host.open_service("gate", gate.clone()).unwrap();
    fixture
        .attachment
        .handle
        .submit(IntentId::mint(), Input::text("go", Origin::surface("test")));
    tokio::time::timeout(Duration::from_secs(15), gate.entered.notified())
        .await
        .unwrap();
    let intent = IntentId::mint();
    fixture.attachment.handle.submit(
        intent.clone(),
        Input::text("/plugins disable fixture", Origin::surface("rpc")),
    );
    tokio::time::timeout(Duration::from_secs(15), async {
        while let Some(frame) = fixture.attachment.events.next().await {
            if let Event::IntentAck {
                intent: ack,
                outcome,
            } = frame.event
                && ack == intent
            {
                assert!(
                    matches!(outcome, IntentOutcome::Applied { .. }),
                    "{outcome:?}"
                );
                return;
            }
        }
        panic!("no switch acknowledgement");
    })
    .await
    .unwrap();
    let catalogue = fixture.host.catalog(CatalogKind::Plugins).await.unwrap();
    assert_eq!(
        catalogue
            .entries
            .iter()
            .find(|row| row.id == "fixture")
            .unwrap()
            .meta["state"]["kind"],
        "off"
    );
    gate.release.notify_one();
    tokio::time::timeout(Duration::from_secs(15), async {
        while let Some(frame) = fixture.attachment.events.next().await {
            if let Event::TurnCompleted { status, .. } = frame.event {
                assert_eq!(status, TurnStatus::Completed);
                return;
            }
        }
        panic!("no completed turn");
    })
    .await
    .unwrap();
    let store =
        bingo_plugin_host::state::Store::new(&Env::rooted(fixture.home.path()).data_dir, "fixture");
    assert_eq!(
        store
            .get("global".into(), "completed".into())
            .await
            .unwrap()
            .unwrap()
            .value,
        json!(true)
    );
    fixture.host.shutdown().await;
}
