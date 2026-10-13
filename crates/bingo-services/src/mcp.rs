//! The live configured MCP server rows.
use async_trait::async_trait;
use bingo_sdk::{HostHandle, ServiceKey, WireService};
use serde_json::{Map, Value};
use std::sync::Arc;

pub static KEY: ServiceKey<dyn Servers> = ServiceKey::new("mcp.servers");
pub const CAPABILITY: &str = "service:mcp.servers";

#[derive(serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
pub struct Call {
    pub session: bingo_sdk::SessionId,
    pub server: String,
    pub tool: String,
    pub args: Value,
}
#[derive(serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
pub struct Connect {
    pub session: bingo_sdk::SessionId,
    pub server: String,
}

#[async_trait]
pub trait Servers: Send + Sync {
    async fn servers(&self) -> Map<String, Value>;
    async fn call(&self, _input: Call) -> Result<bingo_sdk::ToolOutput, bingo_sdk::ServiceError> {
        Err(bingo_sdk::ServiceError("MCP calls are unavailable".into()))
    }
    async fn connect(&self, _server: &str) -> Result<(), bingo_sdk::ServiceError> {
        Err(bingo_sdk::ServiceError(
            "MCP connections are unavailable".into(),
        ))
    }
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

#[derive(serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
struct Answer {
    servers: Map<String, Value>,
}

#[async_trait]
impl WireService for ToWire {
    fn face(&self) -> Option<bingo_sdk::Face> {
        Some(
            bingo_sdk::Face::new("mcp")
                .method::<(), Answer>("servers", false)
                .method::<Call, bingo_sdk::ToolOutput>("call", false)
                .method::<Connect, ()>("connect", true),
        )
    }

    async fn call(&self, method: &str, params: Value) -> Result<Value, bingo_sdk::ServiceError> {
        if method == "call" {
            let input = serde_json::from_value(params)
                .map_err(|e| bingo_sdk::ServiceError(format!("{e}")))?;
            return Ok(serde_json::json!(self.0.call(input).await?));
        }
        if method == "connect" {
            let input: Connect = serde_json::from_value(params)
                .map_err(|e| bingo_sdk::ServiceError(format!("{e}")))?;
            self.0.connect(&input.server).await?;
            return Ok(Value::Null);
        }
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
    async fn call(&self, input: Call) -> Result<bingo_sdk::ToolOutput, bingo_sdk::ServiceError> {
        serde_json::from_value(self.0.call("call", serde_json::json!(input)).await?)
            .map_err(|e| bingo_sdk::ServiceError(e.to_string()))
    }
    async fn connect(&self, server: &str) -> Result<(), bingo_sdk::ServiceError> {
        self.0
            .call(
                "connect",
                serde_json::json!({"server":server,"session":"service"}),
            )
            .await
            .map(|_| ())
    }
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
    #[test]
    fn world_methods_declare_their_unsafe_boundary() {
        let face = wire(Arc::new(Rows)).face().unwrap();
        assert!(!face.find("call").unwrap().unsafe_call);
        assert!(face.find("connect").unwrap().unsafe_call);
        let input: Call = serde_json::from_value(serde_json::json!({"session":"s","server":"local","tool":"echo","args":{"text":"world"}})).unwrap();
        assert_eq!(input.server, "local");
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
