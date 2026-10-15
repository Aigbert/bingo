//! Typed reading and forgetting, shared by commands and panels.
use async_trait::async_trait;
use bingo_sdk::{HostHandle, KernelError, ServiceError, ServiceKey, WireService};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::{path::PathBuf, sync::Arc};
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub enum Status {
    Active,
    Retired,
    Unreadable,
}
impl Status {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Active => "active",
            Self::Retired => "retired",
            Self::Unreadable => "unreadable",
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub enum Outcome {
    Helpful,
    Harmful,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct EntryRow {
    pub id: String,
    pub status: Status,
    pub summary: String,
    pub outcomes: Vec<Outcome>,
    pub error: Option<String>,
    pub tokens: u64,
}
pub type Listing = Vec<EntryRow>;
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
pub struct List {
    pub cwd: PathBuf,
}
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
pub struct Forget {
    pub cwd: PathBuf,
    pub id: String,
}
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
pub struct Read {
    pub cwd: PathBuf,
    pub id: String,
}

pub static KEY: ServiceKey<dyn Service> = ServiceKey::new("experience");
pub const CAPABILITY: &str = "service:experience";
#[async_trait]
pub trait Service: Send + Sync {
    async fn list(&self, input: List) -> Result<Listing, KernelError>;
    async fn forget(&self, input: Forget) -> Result<String, KernelError>;
    async fn read(&self, input: Read) -> Result<String, KernelError>;
}
pub fn wire(service: Arc<dyn Service>) -> Arc<dyn WireService> {
    Arc::new(ToWire(service))
}
pub fn find(host: &HostHandle) -> Option<Arc<dyn Service>> {
    host.service(&KEY).or_else(|| {
        host.service_wire(KEY.name())
            .map(|wire| Arc::new(FromWire(wire)) as Arc<dyn Service>)
    })
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
            bingo_sdk::Face::new("experience")
                .method::<List, Result<Listing, KernelError>>("list", false)
                .method::<Read, Result<String, KernelError>>("read", false)
                .method::<Forget, Result<String, KernelError>>("forget", true),
        )
    }
    async fn call(&self, method: &str, params: Value) -> Result<Value, ServiceError> {
        match method {
            "list" => Ok(json!(self.0.list(decode(params)?).await)),
            "read" => Ok(json!(self.0.read(decode(params)?).await)),
            "forget" => Ok(json!(self.0.forget(decode(params)?).await)),
            _ => Err(ServiceError("expected list, read or forget".into())),
        }
    }
}
impl FromWire {
    async fn invoke<T: serde::de::DeserializeOwned>(
        &self,
        method: &str,
        params: Value,
    ) -> Result<T, KernelError> {
        let value = self.0.call(method, params).await.map_err(|e| {
            bingo_sdk::KernelError::new(bingo_sdk::ErrorCode::Internal, e.to_string())
        })?;
        decode(value).map_err(|e| {
            bingo_sdk::KernelError::new(bingo_sdk::ErrorCode::Internal, e.to_string())
        })?
    }
}
#[async_trait]
impl Service for FromWire {
    async fn list(&self, input: List) -> Result<Listing, KernelError> {
        self.invoke("list", json!(input)).await
    }
    async fn read(&self, input: Read) -> Result<String, KernelError> {
        self.invoke("read", json!(input)).await
    }
    async fn forget(&self, input: Forget) -> Result<String, KernelError> {
        self.invoke("forget", json!(input)).await
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn unknown_row_values_fail_closed() {
        let mut row = fixture("list")["answer"]["Ok"][0].clone();
        row["status"] = json!("invented");
        assert!(serde_json::from_value::<EntryRow>(row).is_err());
    }
    struct Owner;
    fn fixture(method: &str) -> Value {
        serde_json::from_str(match method {
            "list" => include_str!("../fixtures/experience/list.json"),
            "read" => include_str!("../fixtures/experience/read.json"),
            _ => include_str!("../fixtures/experience/forget.json"),
        })
        .unwrap()
    }
    #[async_trait]
    impl Service for Owner {
        async fn list(&self, _: List) -> Result<Listing, KernelError> {
            serde_json::from_value(fixture("list")["answer"]["Ok"].clone())
                .map_err(|e| KernelError::new(bingo_sdk::ErrorCode::Internal, e.to_string()))
        }
        async fn read(&self, _: Read) -> Result<String, KernelError> {
            Ok("file text".into())
        }
        async fn forget(&self, _: Forget) -> Result<String, KernelError> {
            Ok("forgotten".into())
        }
    }
    #[tokio::test]
    async fn fixtures_and_native_or_wire_lookup() {
        use bingo_sdk::testing::ServiceHost;
        let owner: Arc<dyn Service> = Arc::new(Owner);
        assert!(Arc::ptr_eq(
            &find(&ServiceHost::holding(&KEY, owner.clone())).unwrap(),
            &owner
        ));
        assert!(find(&ServiceHost::handle()).is_none());
        let adapter = wire(owner);
        for method in ["list", "read", "forget"] {
            let fixture = fixture(method);
            assert_eq!(
                adapter
                    .call(method, fixture["params"].clone())
                    .await
                    .unwrap(),
                fixture["answer"]
            );
        }
        assert!(adapter.face().unwrap().find("forget").unwrap().unsafe_call);
        assert!(adapter.call("write", Value::Null).await.is_err());
        let host = ServiceHost::handle();
        host.open_service(KEY.name(), adapter).unwrap();
        let service = find(&host).unwrap();
        assert_eq!(
            json!(
                service
                    .list(decode(fixture("list")["params"].clone()).unwrap())
                    .await
                    .unwrap()
            ),
            fixture("list")["answer"]["Ok"]
        );
        assert_eq!(
            service
                .read(decode(fixture("read")["params"].clone()).unwrap())
                .await
                .unwrap(),
            "file text"
        );
        assert_eq!(
            service
                .forget(decode(fixture("forget")["params"].clone()).unwrap())
                .await
                .unwrap(),
            "forgotten"
        );
    }
}
