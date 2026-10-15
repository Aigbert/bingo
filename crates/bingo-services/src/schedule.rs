//! The owner's schedule listing and session-local pending wake.
mod text;
pub use text::visible;

use async_trait::async_trait;
use bingo_sdk::{HostHandle, KernelError, ServiceError, ServiceKey, SessionId, WireService};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::sync::Arc;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct Pending {
    pub at: String,
    pub note: String,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct Row {
    pub id: String,
    pub spec: String,
    pub next_fire: Option<String>,
    pub enabled: bool,
    pub by: String,
    pub mode: Option<String>,
    pub text: String,
    pub session: Option<SessionId>,
}
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct Listing {
    pub entries: Vec<Row>,
    pub holder: String,
    pub trouble: Option<String>,
}
pub static KEY: ServiceKey<dyn Service> = ServiceKey::new("schedule");
pub const CAPABILITY: &str = "service:schedule";
#[async_trait]
pub trait Service: Send + Sync {
    async fn list(&self) -> Result<Listing, KernelError>;
    async fn pending(&self, session: SessionId) -> Result<Option<Pending>, KernelError>;
    async fn pause(&self, id: String) -> Result<String, KernelError>;
    async fn resume(&self, id: String) -> Result<String, KernelError>;
    async fn run(&self, id: String) -> Result<String, KernelError>;
    async fn forget(&self, id: String) -> Result<String, KernelError>;
}
pub fn find(host: &HostHandle) -> Option<Arc<dyn Service>> {
    host.service(&KEY).or_else(|| {
        host.service_wire(KEY.name())
            .map(|wire| Arc::new(FromWire(wire)) as Arc<dyn Service>)
    })
}
pub fn wire(service: Arc<dyn Service>) -> Arc<dyn WireService> {
    Arc::new(ToWire(service))
}
struct ToWire(Arc<dyn Service>);
struct FromWire(Arc<dyn WireService>);
fn decode<T: serde::de::DeserializeOwned>(value: Value) -> Result<T, ServiceError> {
    serde_json::from_value(value).map_err(|e| ServiceError(e.to_string()))
}
#[async_trait]
impl WireService for ToWire {
    fn face(&self) -> Option<bingo_sdk::Face> {
        Some(
            bingo_sdk::Face::new("schedule")
                .method::<(), Result<Listing, KernelError>>("list", false)
                .method::<SessionId, Result<Option<Pending>, KernelError>>("pending", false)
                .method::<String, Result<String, KernelError>>("pause", true)
                .method::<String, Result<String, KernelError>>("resume", true)
                .method::<String, Result<String, KernelError>>("run", true)
                .method::<String, Result<String, KernelError>>("forget", true),
        )
    }
    async fn call(&self, method: &str, params: Value) -> Result<Value, ServiceError> {
        // Wire callers are plugins, never the person at the input boundary.
        bingo_sdk::render::under_input(false, async {
            Ok(match method {
                "list" => json!(self.0.list().await),
                "pending" => json!(self.0.pending(decode(params)?).await),
                "pause" => json!(self.0.pause(decode(params)?).await),
                "resume" => json!(self.0.resume(decode(params)?).await),
                "run" => json!(self.0.run(decode(params)?).await),
                "forget" => json!(self.0.forget(decode(params)?).await),
                _ => return Err(ServiceError("unknown schedule method".into())),
            })
        })
        .await
    }
}
impl FromWire {
    async fn invoke<T: serde::de::DeserializeOwned>(
        &self,
        method: &str,
        params: Value,
    ) -> Result<T, KernelError> {
        let error =
            |e: ServiceError| KernelError::new(bingo_sdk::ErrorCode::Internal, e.to_string());
        decode(self.0.call(method, params).await.map_err(error)?).map_err(error)?
    }
}
#[async_trait]
impl Service for FromWire {
    async fn list(&self) -> Result<Listing, KernelError> {
        self.invoke("list", Value::Null).await
    }
    async fn pending(&self, session: SessionId) -> Result<Option<Pending>, KernelError> {
        self.invoke("pending", json!(session)).await
    }
    async fn pause(&self, id: String) -> Result<String, KernelError> {
        self.invoke("pause", json!(id)).await
    }
    async fn resume(&self, id: String) -> Result<String, KernelError> {
        self.invoke("resume", json!(id)).await
    }
    async fn run(&self, id: String) -> Result<String, KernelError> {
        self.invoke("run", json!(id)).await
    }
    async fn forget(&self, id: String) -> Result<String, KernelError> {
        self.invoke("forget", json!(id)).await
    }
}
#[cfg(test)]
#[path = "schedule_tests.rs"]
mod tests;
