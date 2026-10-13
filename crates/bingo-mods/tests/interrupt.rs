use bingo_core::{Host, HostConfig};
use bingo_mods::ModsPlugin;
use bingo_sdk::*;
use futures::StreamExt;
use serde_json::{Value, json};
use std::{sync::Arc, time::Duration};

struct Offline;
static MANIFEST: PluginManifest = PluginManifest {
    id: "test.offline",
    version: "1",
    uses: &[],
    provides: &[],
    requires: &[],
    config: None,
};
#[async_trait::async_trait]
impl Plugin for Offline {
    fn manifest(&self) -> &'static PluginManifest {
        &MANIFEST
    }
    fn register(&self, registrar: &mut Registrar) -> Result<(), PluginError> {
        registrar.provider(Arc::new(Offline));
        registrar.tool(Arc::new(Offline));
        Ok(())
    }
}
#[async_trait::async_trait]
impl Provider for Offline {
    fn id(&self) -> &str {
        "offline"
    }
    fn endpoint(&self, _: &str) -> EndpointCapabilities {
        EndpointCapabilities::default()
    }
    async fn stream(
        &self,
        _: ModelRequest,
        _: CancellationToken,
    ) -> Result<ModelStream, ProviderError> {
        Ok(Box::pin(futures::stream::iter([
            Ok(ModelEvent::ToolCall {
                id: "call".into(),
                name: "Read".into(),
                input: "{}".into(),
            }),
            Ok(ModelEvent::Finish {
                usage: Usage::default(),
                finish_reason: FinishReason::unified(UnifiedFinish::ToolCalls),
            }),
        ])))
    }
}
#[async_trait::async_trait]
impl Tool for Offline {
    fn spec(&self) -> ToolSpec {
        ToolSpec {
            name: "Read".into(),
            description: "offline".into(),
            input_schema: json!({"type":"object"}),
            meta: Default::default(),
        }
    }
    fn traits(&self, _: &Value) -> ToolTraits {
        ToolTraits::read_only()
    }
    async fn call(&self, _: Value, _: &ToolContext) -> Result<ToolOutput, ToolError> {
        panic!("esc ends the turn while tool.check loops")
    }
}

#[tokio::test]
async fn esc_during_a_tool_check_loop_ends_interrupted_within_the_grace() {
    let home = tempfile::tempdir().unwrap();
    let root = home.path().join("loop");
    std::fs::create_dir(&root).unwrap();
    std::fs::write(
        root.join("plugin.json"),
        json!({"name":"loop","version":"1","module":"main.ts","uses":["session"]}).to_string(),
    )
    .unwrap();
    std::fs::write(root.join("main.ts"), "export function register(on) { on('tool.check', async ($) => { await $.session.extend('entered', true); while(true) {} }); }").unwrap();
    let config = HostConfig::new(Env::rooted(home.path()))
        .with_layer("test", json!({"provider":"offline","model":"offline"}));
    let host = Host::build(
        vec![
            Box::new(Offline),
            Box::new(ModsPlugin::default().with_dirs(vec![root])),
        ],
        config,
    )
    .await
    .unwrap();
    let mut attachment = host
        .open(
            SessionSelector::Create {
                spec: SessionSpec {
                    cwd: home.path().into(),
                    ..Default::default()
                },
            },
            ClientIdentity {
                name: "test".into(),
                surface: "test".into(),
            },
            OpenOptions::default(),
        )
        .await
        .unwrap();
    attachment
        .handle
        .submit(IntentId::mint(), Input::text("go", Origin::surface("test")));
    tokio::time::timeout(Duration::from_secs(10), async {
        while let Some(frame) = attachment.events.next().await {
            if matches!(frame.event, Event::Extension {kind, ..} if kind == "entered") {
                return;
            }
        }
        panic!("stream ended before the loop");
    })
    .await
    .unwrap();
    tokio::time::pause();
    let start = tokio::time::Instant::now();
    attachment
        .handle
        .interrupt(IntentId::mint(), InterruptScope::Head);
    let status = tokio::time::timeout(Duration::from_millis(2001), async {
        while let Some(frame) = attachment.events.next().await {
            if let Event::TurnCompleted { status, .. } = frame.event {
                return status;
            }
        }
        panic!("stream ended before TurnCompleted");
    })
    .await
    .unwrap();
    assert!(
        tokio::time::Instant::now() - start <= Duration::from_secs(2),
        "interrupt elapsed: {:?}",
        tokio::time::Instant::now() - start
    );
    assert_eq!(
        status,
        TurnStatus::Interrupted {
            reason: InterruptReason::UserCancel
        }
    );
    tokio::time::resume();
    host.shutdown().await;
}
