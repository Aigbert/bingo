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
) -> bingo_services::mcp::ServerRow {
    use bingo_services::mcp::{Auth, ServerRow};
    let mut row = ServerRow {
        name: name.into(),
        transport: transport(server),
        status,
        origin: waiting.map(|entry| format!("this project · {}", entry.root.display())),
        auth: auth.map(|auth| match auth {
            bingo_auth_oauth::Status::SignedOut => Auth::SignedOut,
            bingo_auth_oauth::Status::SignedIn { .. } => Auth::SignedIn,
            bingo_auth_oauth::Status::Expired { reason } => Auth::Expired { reason },
        }),
        latency: None,
        sign_in_from: sign_in_from(server),
        stderr: None,
    };
    // Only a row somebody may dial carries what dialling needs; a row
    // waiting for the person's yes carries what that yes would run.
    row.transport = match row.status {
        bingo_services::mcp::Status::Waiting { .. } => row.transport.reviewed(),
        _ if !row.forwardable() => row.transport.redacted(),
        _ => row.transport,
    };
    row
}

fn transport(server: &crate::Server) -> bingo_services::mcp::Transport {
    use bingo_services::mcp::Transport;
    match server {
        crate::Server::Stdio {
            command,
            args,
            env,
            cwd,
            ..
        } => Transport::Stdio {
            command: command.clone(),
            args: args.clone(),
            env: env.clone(),
            cwd: cwd.clone(),
        },
        crate::Server::Http { url, headers, .. } => Transport::Http {
            url: url.clone(),
            headers: headers.clone(),
        },
    }
}

/// Which settings carry a credential, named and never shown.
fn sign_in_from(server: &crate::Server) -> Option<String> {
    let names = match server {
        crate::Server::Stdio { env, .. } => env.keys(),
        crate::Server::Http { headers, .. } => headers.keys(),
    }
    .cloned()
    .collect::<Vec<_>>();
    (!names.is_empty()).then(|| format!("{} from settings", names.join(", ")))
}

#[cfg(test)]
mod tests {
    use super::*;
    use bingo_services::mcp::{Status, Transport};
    /// A server a person switched off is not a row anyone is handed whole:
    /// bingo would not dial it, so nobody learns its arguments, environment
    /// or headers from it — and the answer is live, not a boot snapshot.
    #[tokio::test]
    async fn a_server_switched_off_is_handed_only_its_program() {
        let servers = serde_json::from_value(serde_json::json!({
            "local":{"command":"fixture","args":["--token","s3cret"],"env":{"TOKEN":"s3cret"}},
            "web":{"type":"http","url":"https://me:s3cret@mcp.example.com/mcp?key=s3cret",
                   "headers":{"Authorization":"Bearer s3cret"}}
        }))
        .unwrap();
        let manager = Arc::new(Manager::new(
            bingo_http::client(bingo_http::Bounds::CALL).unwrap(),
            servers,
            &["local".into()],
            std::env::temp_dir(),
        ));
        let rows = Rows::new(manager.clone()).rows().await;
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].status, Status::Disabled);
        assert!(!rows[0].forwardable());
        assert_eq!(rows[0].sign_in_from.as_deref(), Some("TOKEN from settings"));
        assert_eq!(rows[0].transport, rows[0].transport.redacted());
        assert!(!serde_json::to_string(&rows[0]).unwrap().contains("s3cret"));
        assert!(rows[1].forwardable(), "an enabled row is handed whole");
        assert!(serde_json::to_string(&rows[1]).unwrap().contains("s3cret"));

        manager.disable("web").await;
        let rows = Rows::new(manager).rows().await;
        assert_eq!(
            rows[1].transport,
            Transport::Http {
                url: "https://mcp.example.com".into(),
                headers: Default::default()
            }
        );
        assert!(!serde_json::to_string(&rows).unwrap().contains("s3cret"));
    }
    /// The person deciding sees the whole line the repository wrote and the
    /// names of what it is given; the values stay in the file.
    #[test]
    fn a_server_waiting_for_a_yes_is_handed_what_a_yes_would_run() {
        let server: crate::Server = serde_json::from_value(
            serde_json::json!({"command":"./run.sh","args":["--port","9","API_KEY=s3cret"],
                "env":{"TOKEN":"s3cret"},"cwd":"tools"}),
        )
        .unwrap();
        let row = row(
            "repo",
            &server,
            Status::Waiting { changed: false },
            None,
            None,
        );
        assert_eq!(
            row.transport,
            Transport::Stdio {
                command: "./run.sh".into(),
                args: vec!["--port".into(), "9".into(), "API_KEY=s3cret".into()],
                env: [("TOKEN".into(), "…".into())].into(),
                cwd: Some("tools".into())
            }
        );
        assert_eq!(row.transport.line(), "./run.sh --port 9 API_KEY=s3cret");
    }
}
