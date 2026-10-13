//! Only the session summary and extension journal are needed by a prompt.
use async_trait::async_trait;
use bingo_sdk::*;
use serde_json::{Value, json};
use std::{any::Any, path::Path, sync::Arc};

pub fn host(cwd: &Path) -> HostHandle {
    let summary = serde_json::from_value(json!({
        "id":"ses_picture", "cwd":cwd,
        "createdAt":"2026-01-01T00:00:00Z", "updatedAt":"2026-01-01T00:00:00Z"
    }))
    .unwrap();
    HostHandle(Arc::new(PromptHost(summary)))
}

struct PromptHost(SessionSummary);

#[async_trait]
impl HostApi for PromptHost {
    async fn sessions(&self, _: SessionFilter) -> Result<Vec<SessionSummary>, KernelError> {
        Ok(vec![self.0.clone()])
    }
    async fn extend(&self, _: &SessionId, _: &str, _: &str, _: Value) -> Result<(), KernelError> {
        Ok(())
    }
    async fn open(
        &self,
        selector: SessionSelector,
        who: ClientIdentity,
        options: OpenOptions,
    ) -> Result<Attachment, KernelError> {
        testing::NoHost.open(selector, who, options).await
    }
    async fn close(&self, id: &SessionId, reason: CloseReason) -> Result<(), KernelError> {
        testing::NoHost.close(id, reason).await
    }
    async fn delete(&self, id: &SessionId) -> Result<(), KernelError> {
        testing::NoHost.delete(id).await
    }
    async fn deliver(
        &self,
        id: &SessionId,
        intent: IntentId,
        input: Input,
        delivery: Delivery,
    ) -> Result<(), KernelError> {
        testing::NoHost.deliver(id, intent, input, delivery).await
    }
    async fn signal(
        &self,
        id: &SessionId,
        plugin: &str,
        kind: &str,
        payload: Value,
    ) -> Result<(), KernelError> {
        testing::NoHost.signal(id, plugin, kind, payload).await
    }
    async fn catalog(&self, kind: CatalogKind) -> Result<Catalog, KernelError> {
        testing::NoHost.catalog(kind).await
    }
    fn gateway_events(&self) -> GatewayStream {
        testing::NoHost.gateway_events()
    }
    fn service_any(&self, _: &str) -> Option<Arc<dyn Any + Send + Sync>> {
        None
    }
}
