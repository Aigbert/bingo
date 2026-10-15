//! Read-only preview of the files affected by a rewind.
use async_trait::async_trait;
use bingo_sdk::{HostHandle, ServiceError, ServiceKey, SessionId, TurnId, WireService};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{path::PathBuf, sync::Arc};

pub static KEY: ServiceKey<dyn Checkpoints> = ServiceKey::new("checkpoints");
pub const CAPABILITY: &str = "service:checkpoints";
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct Request {
    pub session: SessionId,
    pub from: TurnId,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum Fate {
    Restored,
    Removed,
    Skipped,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct File {
    pub path: PathBuf,
    pub fate: Fate,
}
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct Touched {
    pub files: Vec<File>,
    pub shell_calls: usize,
}
#[async_trait]
pub trait Checkpoints: Send + Sync {
    async fn touched(&self, session: &SessionId, from: &TurnId) -> Result<Touched, ServiceError>;
}
pub fn find(host: &HostHandle) -> Option<Arc<dyn Checkpoints>> {
    host.service(&KEY).or_else(|| {
        host.service_wire(KEY.name())
            .map(|wire| Arc::new(FromWire(wire)) as Arc<dyn Checkpoints>)
    })
}
pub fn wire(service: Arc<dyn Checkpoints>) -> Arc<dyn WireService> {
    Arc::new(ToWire(service))
}
struct ToWire(Arc<dyn Checkpoints>);
struct FromWire(Arc<dyn WireService>);
fn error(e: impl std::fmt::Display) -> ServiceError {
    ServiceError(e.to_string())
}
#[async_trait]
impl WireService for ToWire {
    fn face(&self) -> Option<bingo_sdk::Face> {
        Some(bingo_sdk::Face::new("checkpoints").method::<Request, Touched>("touched", false))
    }
    async fn call(&self, method: &str, params: Value) -> Result<Value, ServiceError> {
        if method != "touched" {
            return Err(error("checkpoints speaks `touched`"));
        }
        let request: Request = serde_json::from_value(params).map_err(error)?;
        serde_json::to_value(self.0.touched(&request.session, &request.from).await?).map_err(error)
    }
}
#[async_trait]
impl Checkpoints for FromWire {
    async fn touched(&self, session: &SessionId, from: &TurnId) -> Result<Touched, ServiceError> {
        let value = self
            .0
            .call(
                "touched",
                serde_json::json!(Request {
                    session: session.clone(),
                    from: from.clone()
                }),
            )
            .await?;
        serde_json::from_value(value).map_err(error)
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use bingo_sdk::testing::ServiceHost;
    fn fixture() -> Value {
        serde_json::from_str(include_str!("../fixtures/checkpoints/touched.json")).unwrap()
    }
    struct Preview;
    #[async_trait]
    impl Checkpoints for Preview {
        async fn touched(
            &self,
            session: &SessionId,
            from: &TurnId,
        ) -> Result<Touched, ServiceError> {
            assert_eq!(session.as_str(), "ses_1");
            assert_eq!(from.as_str(), "trn_1");
            Ok(serde_json::from_value(fixture()["answer"].clone()).unwrap())
        }
    }
    #[tokio::test]
    async fn fixture_round_trips_native_and_wire() {
        let fixture = fixture();
        let service: Arc<dyn Checkpoints> = Arc::new(Preview);
        let native = ServiceHost::holding(&KEY, service.clone());
        assert!(Arc::ptr_eq(&find(&native).unwrap(), &service));
        let face = wire(service);
        assert!(!face.face().unwrap().find("touched").unwrap().unsafe_call);
        assert_eq!(
            face.call("touched", fixture["params"].clone())
                .await
                .unwrap(),
            fixture["answer"]
        );
        assert!(face.call("apply", Value::Null).await.is_err());
        let host = ServiceHost::handle();
        host.open_service(KEY.name(), face).unwrap();
        assert_eq!(
            serde_json::to_value(
                find(&host)
                    .unwrap()
                    .touched(&SessionId::from_raw("ses_1"), &TurnId::from_raw("trn_1"))
                    .await
                    .unwrap()
            )
            .unwrap(),
            fixture["answer"]
        );
        assert!(find(&ServiceHost::handle()).is_none());
    }
}
