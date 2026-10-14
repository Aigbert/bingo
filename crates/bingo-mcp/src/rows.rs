//! The live MCP configuration behind the shared service contract.
use crate::manager::Manager;
use async_trait::async_trait;
use bingo_services::mcp::Servers;

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
    async fn call(
        &self,
        input: bingo_services::mcp::Call,
    ) -> Result<bingo_sdk::ToolOutput, bingo_sdk::ServiceError> {
        self.manager.call(input).await
    }
    async fn connect(&self, server: &str) -> Result<(), bingo_sdk::ServiceError> {
        self.manager.connect(server).await
    }
    async fn rows(&self) -> Vec<bingo_services::mcp::ServerRow> {
        self.manager.rows().await
    }
}

#[cfg(test)]
mod world_contract {
    use super::*;
    #[tokio::test]
    async fn unknown_servers_are_refused_without_dialling() {
        let manager = Arc::new(Manager::new(
            bingo_http::client(bingo_http::Bounds::CALL).unwrap(),
            Default::default(),
            &[],
            std::env::temp_dir(),
        ));
        let rows = Rows::new(manager);
        assert!(
            rows.connect("unconfigured")
                .await
                .unwrap_err()
                .to_string()
                .contains("not configured")
        );
    }
}

pub(crate) fn row(
    name: &str,
    server: &crate::Server,
    status: bingo_services::mcp::Status,
    auth: Option<bingo_auth_oauth::Status>,
    waiting: Option<&bingo_sdk::Waiting>,
) -> Option<bingo_services::mcp::ServerRow> {
    use bingo_services::mcp::{Auth, ServerRow};
    let transport = serde_json::from_value(crate::config::row(server)).ok()?;
    let sign_in_from = match server {
        crate::Server::Stdio { env, .. } if !env.is_empty() => Some(format!(
            "{} from settings",
            env.keys().cloned().collect::<Vec<_>>().join(", ")
        )),
        crate::Server::Http { headers, .. } if !headers.is_empty() => Some(format!(
            "{} from settings",
            headers.keys().cloned().collect::<Vec<_>>().join(", ")
        )),
        _ => None,
    };
    Some(ServerRow {
        name: name.into(),
        transport,
        status,
        origin: waiting.map(|entry| format!("this project · {}", entry.root.display())),
        auth: auth.map(|auth| match auth {
            bingo_auth_oauth::Status::SignedOut => Auth::SignedOut,
            bingo_auth_oauth::Status::SignedIn { .. } => Auth::SignedIn,
            bingo_auth_oauth::Status::Expired { reason } => Auth::Expired { reason },
        }),
        latency: None,
        sign_in_from,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use bingo_services::mcp::{Status, Transport};
    #[tokio::test]
    async fn rows_include_disabled_servers_but_forbid_forwarding_them() {
        let servers = serde_json::from_value(
            serde_json::json!({"local":{"command":"fixture","env":{"TOKEN":"secret"}}}),
        )
        .unwrap();
        let manager = Arc::new(Manager::new(
            bingo_http::client(bingo_http::Bounds::CALL).unwrap(),
            servers,
            &["local".into()],
            std::env::temp_dir(),
        ));
        let rows = Rows::new(manager).rows().await;
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].status, Status::Disabled);
        assert!(!rows[0].forwardable());
        assert_eq!(rows[0].sign_in_from.as_deref(), Some("TOKEN from settings"));
        assert!(matches!(rows[0].transport, Transport::Stdio { .. }));
        assert!(!format!("{:?}", rows[0]).contains("secret"));
    }
}
