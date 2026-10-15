//! Typed rooms-control operations and their process boundary.
use async_trait::async_trait;
use bingo_sdk::{Face, HostHandle, ServiceError, ServiceKey, SessionId, WireService};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::sync::Arc;
pub static KEY: ServiceKey<dyn RoomsControl> = ServiceKey::new("rooms-control");
pub const CAPABILITY: &str = "service:rooms-control";
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct RoomRow {
    pub id: SessionId,
    pub name: String,
    pub purpose: Option<String>,
    pub seats: Vec<String>,
    pub holder: String,
    pub closed: bool,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct Session {
    pub session: SessionId,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct Named {
    pub session: SessionId,
    pub name: String,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct Seat {
    pub session: SessionId,
    pub name: String,
    pub agent: String,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct Purpose {
    pub session: SessionId,
    pub name: String,
    pub text: String,
}
#[async_trait]
pub trait RoomsControl: Send + Sync {
    async fn rooms(&self, request: Session) -> Result<Vec<RoomRow>, ServiceError>;
    async fn seat(&self, request: Seat) -> Result<String, ServiceError>;
    async fn unseat(&self, request: Seat) -> Result<String, ServiceError>;
    async fn set_purpose(&self, request: Purpose) -> Result<String, ServiceError>;
    async fn close(&self, request: Named) -> Result<String, ServiceError>;
}
pub fn find(host: &HostHandle) -> Option<Arc<dyn RoomsControl>> {
    host.service(&KEY).or_else(|| {
        host.service_wire(KEY.name())
            .map(|w| Arc::new(FromWire(w)) as Arc<dyn RoomsControl>)
    })
}
pub fn wire(owner: Arc<dyn RoomsControl>) -> Arc<dyn WireService> {
    Arc::new(ToWire(owner))
}
struct ToWire(Arc<dyn RoomsControl>);
struct FromWire(Arc<dyn WireService>);
fn error(e: impl std::fmt::Display) -> ServiceError {
    ServiceError(e.to_string())
}
#[async_trait]
impl WireService for ToWire {
    fn face(&self) -> Option<Face> {
        let mut face = Face::new("rooms-control")
            .method::<Session, Vec<RoomRow>>("rooms", false)
            .method::<Seat, String>("seat", false)
            .method::<Seat, String>("unseat", false)
            .method::<Purpose, String>("set_purpose", false)
            .method::<Named, String>("close", false);
        for method in &mut face.methods {
            method.entry.pinned = &["/session"];
        }
        Some(face)
    }
    async fn call(&self, method: &str, params: Value) -> Result<Value, ServiceError> {
        match method {
            "rooms" => serde_json::to_value(
                self.0
                    .rooms(serde_json::from_value(params).map_err(error)?)
                    .await?,
            )
            .map_err(error),
            "seat" => serde_json::to_value(
                self.0
                    .seat(serde_json::from_value(params).map_err(error)?)
                    .await?,
            )
            .map_err(error),
            "unseat" => serde_json::to_value(
                self.0
                    .unseat(serde_json::from_value(params).map_err(error)?)
                    .await?,
            )
            .map_err(error),
            "set_purpose" => serde_json::to_value(
                self.0
                    .set_purpose(serde_json::from_value(params).map_err(error)?)
                    .await?,
            )
            .map_err(error),
            "close" => serde_json::to_value(
                self.0
                    .close(serde_json::from_value(params).map_err(error)?)
                    .await?,
            )
            .map_err(error),
            other => Err(error(format!("unknown method: {other}"))),
        }
    }
}
#[async_trait]
impl RoomsControl for FromWire {
    async fn rooms(&self, request: Session) -> Result<Vec<RoomRow>, ServiceError> {
        let answer = self
            .0
            .call("rooms", serde_json::to_value(request).map_err(error)?)
            .await?;
        serde_json::from_value(answer).map_err(error)
    }
    async fn seat(&self, request: Seat) -> Result<String, ServiceError> {
        let answer = self
            .0
            .call("seat", serde_json::to_value(request).map_err(error)?)
            .await?;
        serde_json::from_value(answer).map_err(error)
    }
    async fn unseat(&self, request: Seat) -> Result<String, ServiceError> {
        let answer = self
            .0
            .call("unseat", serde_json::to_value(request).map_err(error)?)
            .await?;
        serde_json::from_value(answer).map_err(error)
    }
    async fn set_purpose(&self, request: Purpose) -> Result<String, ServiceError> {
        let answer = self
            .0
            .call("set_purpose", serde_json::to_value(request).map_err(error)?)
            .await?;
        serde_json::from_value(answer).map_err(error)
    }
    async fn close(&self, request: Named) -> Result<String, ServiceError> {
        let answer = self
            .0
            .call("close", serde_json::to_value(request).map_err(error)?)
            .await?;
        serde_json::from_value(answer).map_err(error)
    }
}
#[cfg(test)]
mod tests;
