//! The live configured MCP server rows.
use async_trait::async_trait;
use bingo_sdk::{HostHandle, ServiceKey, WireService};
use serde_json::{Map, Value};
use std::sync::Arc;

pub static KEY: ServiceKey<dyn Servers> = ServiceKey::new("mcp.servers");
pub const CAPABILITY: &str = "service:mcp.servers";

#[async_trait]
pub trait Servers: Send + Sync {
    async fn servers(&self) -> Map<String, Value>;
}

pub fn wire(service: Arc<dyn Servers>) -> Arc<dyn WireService> {
    Arc::new(ToWire(service))
}

pub fn find(host: &HostHandle) -> Option<Arc<dyn Servers>> {
    host.service(&KEY).or_else(|| {
        host.service_wire(KEY.name())
            .map(|wire| Arc::new(FromWire(wire)) as Arc<dyn Servers>)
    })
}

struct ToWire(Arc<dyn Servers>);

#[async_trait]
impl WireService for ToWire {
    async fn call(&self, method: &str, _params: Value) -> Result<Value, bingo_sdk::ServiceError> {
        if method != "servers" {
            return Err(bingo_sdk::ServiceError(format!(
                "{} speaks `servers`, not `{method}`",
                KEY.name()
            )));
        }
        Ok(serde_json::json!({ "servers": self.0.servers().await }))
    }
}

struct FromWire(Arc<dyn WireService>);

#[async_trait]
impl Servers for FromWire {
    async fn servers(&self) -> Map<String, Value> {
        let Ok(answer) = self.0.call("servers", Value::Null).await else {
            return Map::new();
        };
        answer["servers"].as_object().cloned().unwrap_or_default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bingo_sdk::testing::ServiceHost;
    struct Rows;
    fn fixture() -> Value {
        serde_json::from_str(include_str!("../fixtures/mcp.json")).unwrap()
    }
    #[async_trait]
    impl Servers for Rows {
        async fn servers(&self) -> Map<String, Value> {
            fixture()["answer"]["servers"].as_object().unwrap().clone()
        }
    }
    #[tokio::test]
    async fn the_mcp_wire_contract_round_trips() {
        let recorded = fixture();
        let face = wire(Arc::new(Rows));
        assert_eq!(
            face.call(
                recorded["method"].as_str().unwrap(),
                recorded["params"].clone()
            )
            .await
            .unwrap(),
            recorded["answer"]
        );
        assert!(
            face.call("tools", Value::Null)
                .await
                .unwrap_err()
                .to_string()
                .contains("servers")
        );
        let host = ServiceHost::handle();
        host.open_service(KEY.name(), face).unwrap();
        assert_eq!(
            Value::Object(find(&host).unwrap().servers().await),
            recorded["answer"]["servers"]
        );
    }
    #[test]
    fn a_typed_mcp_owner_is_found_without_a_wire_face() {
        let owner: Arc<dyn Servers> = Arc::new(Rows);
        let host = ServiceHost::holding(&KEY, owner.clone());
        assert!(Arc::ptr_eq(&find(&host).unwrap(), &owner));
        assert!(find(&ServiceHost::handle()).is_none());
    }
}
