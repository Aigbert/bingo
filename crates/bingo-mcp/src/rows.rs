//! The live MCP configuration behind the shared service contract.
use crate::manager::Manager;
use async_trait::async_trait;
use bingo_services::mcp::Servers;
use serde_json::{Map, Value};
use std::sync::Arc;

pub struct Rows {
    manager: Arc<Manager>,
}
impl Rows {
    pub fn new(manager: Arc<Manager>) -> Self {
        Self { manager }
    }
}
#[async_trait]
impl Servers for Rows {
    async fn servers(&self) -> Map<String, Value> {
        self.manager.rows().await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bingo_services::mcp;
    use serde_json::{Value, json};
    const METHOD: &str = "servers";
    fn wire(manager: Arc<Manager>) -> Arc<dyn bingo_sdk::WireService> {
        mcp::wire(Arc::new(Rows::new(manager)))
    }
    use crate::config::Server;
    use std::collections::BTreeMap;

    fn manager(servers: Value, disabled: &[String]) -> Arc<Manager> {
        let servers: BTreeMap<String, Server> =
            serde_json::from_value(servers).expect("readable rows");
        Arc::new(Manager::new(
            bingo_http::client(bingo_http::Bounds::CALL).expect("a client"),
            servers,
            disabled,
            std::env::temp_dir(),
        ))
    }

    /// The recorded answer: the `mcpServers` key's own shape, keyed by name,
    /// with what dialling each server takes.
    #[tokio::test]
    async fn the_service_answers_the_rows_as_a_person_wrote_them() {
        let rows = wire(manager(
            json!({
                "files": { "command": "npx", "args": ["-y", "files"], "env": { "TOKEN": "s3cret" } },
                "remote": { "type": "http", "url": "https://mcp.example.com/mcp" }
            }),
            &[],
        ));
        let answered = rows.call(METHOD, Value::Null).await.expect("it answers");
        assert_eq!(
            answered["servers"]["files"],
            json!({ "type": "stdio", "command": "npx", "args": ["-y", "files"],
                    "env": { "TOKEN": "s3cret" }, "cwd": null })
        );
        assert_eq!(
            answered["servers"]["remote"],
            json!({ "type": "http", "url": "https://mcp.example.com/mcp", "headers": {} })
        );
    }

    /// A server a person switched off is not a row anyone is handed: bingo
    /// would not dial it either.
    #[tokio::test]
    async fn a_server_that_is_switched_off_is_not_answered() {
        let manager = manager(
            json!({ "files": { "command": "npx" }, "web": { "command": "npx" } }),
            &["web".to_string()],
        );
        let rows = wire(Arc::clone(&manager));
        let answered = rows.call(METHOD, Value::Null).await.expect("it answers");
        assert!(answered["servers"]["files"].is_object());
        assert!(answered["servers"].get("web").is_none());

        manager.disable("files").await;
        let answered = rows.call(METHOD, Value::Null).await.expect("it answers");
        assert_eq!(
            answered["servers"],
            json!({}),
            "the answer is live, not a snapshot taken at boot"
        );
    }

    #[tokio::test]
    async fn a_method_this_service_does_not_speak_says_which_one_it_does() {
        let refused = wire(manager(json!({}), &[]))
            .call("tools", Value::Null)
            .await
            .expect_err("a refusal");
        assert!(refused.to_string().contains("servers"), "{refused}");
    }
}
