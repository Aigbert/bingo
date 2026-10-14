//! The MCP client: configured servers dialled in the background, their tools
//! offered to the model untrusted, their stderr sent to a log.
//!
//! Registration is synchronous and does no I/O, so the plugin contributes a
//! [`ToolSource`] rather than tools: `start` puts one dial per enabled server
//! on its own task and returns at once, and whatever has landed by the time a
//! turn begins is that turn's tool set (ADR-0009 §1). The first turn of a
//! session may therefore run before a slow server arrives — that is the
//! design; the alternative is a session that will not start until the slowest
//! server on the list has answered.
//!
//! Nothing a server says about itself is believed: an MCP tool's traits are
//! the fail-closed default, so the gate asks about every call (ADR-0009 §2).

pub mod auth;
pub mod client;
pub mod command;
pub mod config;
pub mod dial;
pub mod elicitation;
pub mod guide;
pub mod manager;
pub mod rows;
pub mod source;
pub mod tool;

use std::sync::{Arc, OnceLock};

use async_trait::async_trait;
use bingo_sdk::{
    Command, ConfigClaim, Contribution, HostHandle, Merge, Plugin, PluginError, PluginManifest,
    Registrar, ToolSource,
};

use bingo_services::mcp;
pub use client::{Asker, Client};
pub use command::McpCommand;
pub use config::{Server, Settings};
pub use dial::CONNECT_TIMEOUT;
pub use manager::{Line, Manager, Status};
pub use rows::Rows;
pub use source::McpSource;
pub use tool::{McpTool, tool_name};

pub const SETTING: &str = "mcpServers";

pub static MANIFEST: PluginManifest = PluginManifest {
    id: "bingo.mcp",
    version: env!("CARGO_PKG_VERSION"),
    uses: &[],
    provides: &[
        "tools:mcp",
        "plugins:mcp",
        "command:mcp",
        mcp::CAPABILITY,
        "service:bingo.mcp.pages",
    ],
    requires: &[],
    config: Some(ConfigClaim {
        approve: &[SETTING],
        keys: &[
            (SETTING, Merge::ByName),
            ("disabledMcpServers", Merge::Accumulate),
        ],
        schema,
    }),
};

fn schema() -> schemars::Schema {
    schemars::schema_for!(Settings)
}

/// Registers the tool source and `/mcp`, and dials the servers on `start`.
#[derive(Debug, Default)]
pub struct McpPlugin {
    /// Built in `register`, where the settings are, and used by `start` and
    /// `stop`, which are handed nothing but the host.
    manager: OnceLock<Arc<Manager>>,
}

#[async_trait]
impl Plugin for McpPlugin {
    fn manifest(&self) -> &'static PluginManifest {
        &MANIFEST
    }

    fn register(&self, registrar: &mut Registrar) -> Result<(), PluginError> {
        let settings: Settings = registrar.config()?;
        let manager = Arc::new(
            Manager::new(
                bingo_http::client(bingo_http::Bounds::CALL)
                    .map_err(|e| PluginError::Failed(e.to_string()))?,
                settings.mcp_servers,
                &settings.disabled_mcp_servers,
                registrar.env().data_dir.clone(),
            )
            .with_waiting(settings.waiting),
        );
        registrar.add(Contribution::Tools(
            Arc::new(McpSource::new(Arc::clone(&manager))) as Arc<dyn ToolSource>,
        ));
        registrar.add(Contribution::Command(
            Arc::new(McpCommand::new(Arc::clone(&manager))) as Arc<dyn Command>,
        ));
        registrar.add(service(Arc::clone(&manager)));
        registrar.add(guide::contribution(registrar));
        registrar.add(Contribution::Plugins(manager.clone()));
        self.manager
            .set(manager)
            .map_err(|_| PluginError::Failed("the mcp plugin registered twice".into()))
    }

    /// Returns the moment the dialling is on its own task, not when it lands.
    async fn start(&self, host: HostHandle) -> Result<(), PluginError> {
        let Some(manager) = self.manager.get().cloned() else {
            return Ok(());
        };
        manager.bind(&host);
        tokio::spawn(async move { manager.dial_enabled().await });
        Ok(())
    }

    async fn stop(&self) -> Result<(), PluginError> {
        if let Some(manager) = self.manager.get() {
            manager.shutdown().await;
        }
        Ok(())
    }
}

/// Both faces read the same live manager.
fn service(manager: Arc<Manager>) -> Contribution {
    let owner: Arc<dyn mcp::Servers> = Arc::new(Rows::new(manager));
    Contribution::service(&mcp::KEY, owner.clone(), Some(mcp::wire(owner)))
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use serde_json::json;

    /// A door that answers from a script: a question put where no surface is
    /// listening.
    #[derive(Debug, Default)]
    pub(crate) struct Scripted {
        answers: std::sync::Mutex<std::collections::VecDeque<bingo_sdk::Answer>>,
    }

    impl Scripted {
        pub(crate) fn new(answers: Vec<bingo_sdk::Answer>) -> Arc<Self> {
            Arc::new(Self {
                answers: std::sync::Mutex::new(answers.into()),
            })
        }
    }

    #[async_trait]
    impl bingo_sdk::Prompter for Scripted {
        async fn ask(
            &self,
            _kind: bingo_sdk::InteractionKind,
            _answers: Vec<bingo_sdk::AnswerSpec>,
        ) -> Result<bingo_sdk::Answer, bingo_sdk::KernelError> {
            let next = self.answers.lock().ok().and_then(|mut a| a.pop_front());
            next.ok_or_else(|| {
                bingo_sdk::KernelError::new(
                    bingo_sdk::ErrorCode::Internal,
                    "nobody is at this session",
                )
            })
        }
    }

    #[async_trait]
    impl bingo_sdk::ToolHost for Scripted {
        fn progress(&self, _item: &bingo_sdk::ItemId, _tail: String) {}

        async fn record(
            &self,
            _body: bingo_sdk::ItemBody,
        ) -> Result<bingo_sdk::ItemId, bingo_sdk::KernelError> {
            Ok(bingo_sdk::ItemId::from_raw("itm_test"))
        }
    }

    fn registrar(config: serde_json::Value) -> Registrar {
        Registrar::new(
            "bingo.mcp",
            config,
            bingo_sdk::Env::rooted("/tmp"),
            Default::default(),
        )
    }

    #[test]
    fn the_manifest_says_what_it_provides_and_what_it_claims() {
        assert_eq!(MANIFEST.id, "bingo.mcp");
        assert!(MANIFEST.requires.is_empty());
        let claim = MANIFEST.config.expect("a config claim");
        assert_eq!(claim.keys[0], (SETTING, Merge::ByName));
        assert_eq!(claim.keys[1], ("disabledMcpServers", Merge::Accumulate));
    }

    #[test]
    fn the_plugin_registers_a_tool_source_a_command_and_the_rows() {
        let mut registrar = registrar(json!({
            "mcpServers": { "files": { "command": "npx", "args": ["-y", "files"] } }
        }));
        McpPlugin::default()
            .register(&mut registrar)
            .expect("register");
        let contributions = registrar.into_contributions();
        assert_eq!(contributions.len(), MANIFEST.provides.len());
        match &contributions[0] {
            Contribution::Tools(source) => assert_eq!(source.id(), "mcp"),
            other => panic!("expected a tool source, got {other:?}"),
        }
        match &contributions[1] {
            Contribution::Command(command) => assert_eq!(command.spec().name, "mcp"),
            other => panic!("expected a command, got {other:?}"),
        }
        match &contributions[2] {
            Contribution::Service { key, wire, .. } => {
                assert_eq!(key, mcp::KEY.name());
                assert!(
                    wire.is_some(),
                    "the typed and wire faces share one live owner"
                );
            }
            other => panic!("expected the rows service, got {other:?}"),
        }
        match &contributions[3] {
            Contribution::Service { key, wire, .. } => {
                assert_eq!(key, bingo_sdk::Pages::key(MANIFEST.id).name());
                assert!(wire.is_none(), "a page is read in process");
            }
            other => panic!("expected the page service, got {other:?}"),
        }
    }

    #[test]
    fn a_server_this_crate_cannot_read_stops_the_plugin_rather_than_being_dropped() {
        let mut registrar = registrar(json!({
            "mcpServers": { "legacy": { "type": "sse", "url": "http://localhost:8000/sse" } }
        }));
        let error = McpPlugin::default()
            .register(&mut registrar)
            .expect_err("refused");
        assert!(matches!(error, PluginError::Config(_)), "{error}");
        assert!(error.to_string().contains("sse"), "{error}");
    }

    #[test]
    fn the_claimed_schema_describes_both_keys() {
        let schema = serde_json::to_value(schema()).expect("a schema is json");
        let properties = &schema["properties"];
        assert!(properties.get("mcpServers").is_some(), "{schema}");
        assert!(properties.get("disabledMcpServers").is_some(), "{schema}");
    }

    #[tokio::test]
    async fn a_plugin_that_registered_nothing_starts_and_stops_quietly() {
        let plugin = McpPlugin::default();
        plugin.stop().await.expect("stop");
        let mut registrar = registrar(json!({}));
        plugin.register(&mut registrar).expect("register");
        plugin.stop().await.expect("stop");
    }
}
