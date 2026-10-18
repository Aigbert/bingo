use bingo_sdk::*;
use serde_json::{Value, json};
use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};
#[derive(Default)]
pub struct Offline(AtomicUsize);
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
        registrar.provider(Arc::new(Offline::default()));
        registrar.tool(Arc::new(Offline::default()));
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
        let mut events = Vec::new();
        let finish = if self.0.fetch_add(1, Ordering::Relaxed) == 0 {
            events.push(Ok(ModelEvent::ToolCall {
                id: "call".into(),
                name: "Read".into(),
                input: "{}".into(),
            }));
            UnifiedFinish::ToolCalls
        } else {
            UnifiedFinish::Stop
        };
        events.push(Ok(ModelEvent::Finish {
            usage: Usage::default(),
            finish_reason: FinishReason::unified(finish),
        }));
        Ok(Box::pin(futures::stream::iter(events)))
    }
}
#[async_trait::async_trait]
impl Tool for Offline {
    fn spec(&self) -> ToolSpec {
        ToolSpec {
            look: None,
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
        Ok(ToolOutput::text("below"))
    }
}
