//! Typed tasks-write operations and their process boundary.
use async_trait::async_trait;
use bingo_sdk::{Face, HostHandle, ServiceError, ServiceKey, SessionId, WireService};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::sync::Arc;
pub static KEY: ServiceKey<dyn TasksWrite> = ServiceKey::new("tasks-write");
pub const CAPABILITY: &str = "service:tasks-write";
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct TaskRow {
    pub id: u64,
    pub subject: String,
    pub status: Status,
    pub owner: Option<String>,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct Session {
    pub session: SessionId,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct Add {
    pub session: SessionId,
    pub subject: String,
    pub owner: Option<String>,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct SetStatus {
    pub session: SessionId,
    pub id: u64,
    pub status: Status,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct Reassign {
    pub session: SessionId,
    pub id: u64,
    pub owner: String,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Status {
    Pending,
    InProgress,
    Completed,
}
#[async_trait]
pub trait TasksWrite: Send + Sync {
    async fn board(&self, request: Session) -> Result<Vec<TaskRow>, ServiceError>;
    async fn add(&self, request: Add) -> Result<String, ServiceError>;
    async fn set_status(&self, request: SetStatus) -> Result<String, ServiceError>;
    async fn reassign(&self, request: Reassign) -> Result<String, ServiceError>;
}
pub fn find(host: &HostHandle) -> Option<Arc<dyn TasksWrite>> {
    host.service(&KEY).or_else(|| {
        host.service_wire(KEY.name())
            .map(|w| Arc::new(FromWire(w)) as Arc<dyn TasksWrite>)
    })
}
pub fn wire(owner: Arc<dyn TasksWrite>) -> Arc<dyn WireService> {
    Arc::new(ToWire(owner))
}
struct ToWire(Arc<dyn TasksWrite>);
struct FromWire(Arc<dyn WireService>);
fn error(e: impl std::fmt::Display) -> ServiceError {
    ServiceError(e.to_string())
}
#[async_trait]
impl WireService for ToWire {
    fn face(&self) -> Option<Face> {
        let mut face = Face::new("tasks-write")
            .method::<Session, Vec<TaskRow>>("board", false)
            .method::<Add, String>("add", false)
            .method::<SetStatus, String>("set_status", false)
            .method::<Reassign, String>("reassign", false);
        for method in &mut face.methods {
            method.entry.pinned = &["/session"];
        }
        Some(face)
    }
    async fn call(&self, method: &str, params: Value) -> Result<Value, ServiceError> {
        match method {
            "board" => serde_json::to_value(
                self.0
                    .board(serde_json::from_value(params).map_err(error)?)
                    .await?,
            )
            .map_err(error),
            "add" => serde_json::to_value(
                self.0
                    .add(serde_json::from_value(params).map_err(error)?)
                    .await?,
            )
            .map_err(error),
            "set_status" => serde_json::to_value(
                self.0
                    .set_status(serde_json::from_value(params).map_err(error)?)
                    .await?,
            )
            .map_err(error),
            "reassign" => serde_json::to_value(
                self.0
                    .reassign(serde_json::from_value(params).map_err(error)?)
                    .await?,
            )
            .map_err(error),
            other => Err(error(format!("unknown method: {other}"))),
        }
    }
}
#[async_trait]
impl TasksWrite for FromWire {
    async fn board(&self, request: Session) -> Result<Vec<TaskRow>, ServiceError> {
        let answer = self
            .0
            .call("board", serde_json::to_value(request).map_err(error)?)
            .await?;
        serde_json::from_value(answer).map_err(error)
    }
    async fn add(&self, request: Add) -> Result<String, ServiceError> {
        let answer = self
            .0
            .call("add", serde_json::to_value(request).map_err(error)?)
            .await?;
        serde_json::from_value(answer).map_err(error)
    }
    async fn set_status(&self, request: SetStatus) -> Result<String, ServiceError> {
        let answer = self
            .0
            .call("set_status", serde_json::to_value(request).map_err(error)?)
            .await?;
        serde_json::from_value(answer).map_err(error)
    }
    async fn reassign(&self, request: Reassign) -> Result<String, ServiceError> {
        let answer = self
            .0
            .call("reassign", serde_json::to_value(request).map_err(error)?)
            .await?;
        serde_json::from_value(answer).map_err(error)
    }
}
#[cfg(test)]
mod tests;
