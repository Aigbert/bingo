use bingo_sdk::*;
use futures::channel::mpsc;
use serde_json::json;
use std::sync::Arc;

pub fn host() -> (testing::TestHost, CommandContext) {
    let host = testing::TestHost::default();
    host.on_open(|_, _, _| async {
        let snapshot = snapshot();
        let (tx, rx) = mpsc::unbounded();
        Ok(Attachment {
            session: snapshot.summary.id.clone(),
            snapshot,
            history: None,
            events: Box::pin(rx),
            handle: SessionHandle(Arc::new(AckPort(tx))),
        })
    });
    host.on_signal(|_, _, _, _| async { Ok(()) });
    let cx = CommandContext {
        session: SessionId::from_raw("picker"),
        cwd: ".".into(),
        host: host.handle(),
        cancel: CancellationToken::new(),
    };
    (host, cx)
}

pub fn snapshot() -> SessionState {
    SessionState::new(
        serde_json::from_value(json!({
            "id":"picker", "cwd":".", "provider":"p", "model":"first",
            "createdAt":"2026-01-01T00:00:00Z", "updatedAt":"2026-01-01T00:00:00Z",
            "usage":Usage::default()
        }))
        .unwrap(),
    )
}

struct AckPort(mpsc::UnboundedSender<Frame>);
#[async_trait::async_trait]
impl SessionPort for AckPort {
    fn submit(&self, intent: IntentId, _: Input) {
        self.0
            .unbounded_send(Frame {
                cause: None,
                session: SessionId::from_raw("picker"),
                seq: Seq(1),
                ts: snapshot().summary.created_at,
                event: Event::IntentAck {
                    intent,
                    outcome: IntentOutcome::Applied { result: json!({}) },
                },
            })
            .unwrap();
    }
    fn interrupt(&self, _: IntentId, _: InterruptScope) {
        panic!("unexpected interrupt");
    }
    fn answer(&self, _: IntentId, _: InteractionId, _: Answer, _: Activation) {
        panic!("unexpected answer");
    }
    async fn history(&self, _: HistoryPage) -> Result<HistoryChunk, KernelError> {
        panic!("unexpected history");
    }
    async fn events_since(&self, _: Seq) -> Result<FrameStream, KernelError> {
        panic!("unexpected replay");
    }
}

pub fn entries() -> (Vec<CatalogEntry>, Vec<CatalogEntry>) {
    let models = ["first", "second"]
        .into_iter()
        .map(|name| CatalogEntry {
            id: format!("p/{name}"),
            label: name.into(),
            meta: json!({"provider":"p","pinned":true,"context":128000}),
        })
        .collect();
    let providers = vec![CatalogEntry {
        id: "p".into(),
        label: "p".into(),
        meta: json!({"auth":{"kind":"ready"},"reach":{"kind":"ok"}}),
    }];
    (models, providers)
}
