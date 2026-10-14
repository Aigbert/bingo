//! The composed binary publishes only tool schemas that fit the event envelope.
#![allow(clippy::unwrap_used, clippy::expect_used)]
use bingo_sdk::{CatalogKind, HostApi, catalogue::reserved_tool_field};
use serde_json::json;
mod support;

#[tokio::test(flavor = "multi_thread")]
async fn built_in_tool_inputs_leave_the_envelope_fields_free() {
    let mut server = support::Server::spawn(r#"{"responses":[]}"#);
    let kernel = server.kernel();
    kernel.initialize(support::who()).await.unwrap();
    let catalog = kernel.catalog(CatalogKind::Tools).await.unwrap();
    assert!(!catalog.entries.is_empty());
    let mut schemas: Vec<_> = catalog
        .entries
        .into_iter()
        .map(|entry| (entry.id, entry.meta["inputSchema"].clone()))
        .collect();
    if std::env::var_os("BINGO_TEST_RESERVED_TOOL").is_some() {
        schemas.push((
            "fixture".into(),
            json!({"properties":{"tool_use_id":{"type":"string"}}}),
        ));
    }
    for (tool, schema) in schemas {
        assert_eq!(
            reserved_tool_field(&schema),
            None,
            "{tool} collides with the tool.call envelope"
        );
    }
    kernel.shutdown().await.unwrap();
}

struct FaceWatch(std::sync::Arc<std::sync::Mutex<Vec<String>>>);
#[async_trait::async_trait]
impl bingo_sdk::Interceptor for FaceWatch {
    fn id(&self) -> &str {
        "face-watch"
    }
    fn matcher(&self) -> bingo_sdk::LinkMatcher {
        bingo_sdk::LinkMatcher {
            render: None,
            patterns: vec![bingo_sdk::catalogue::Pattern::parse("mcp.*").unwrap()],
            prefilter: serde_json::Value::Null,
        }
    }
    async fn chain(
        &self,
        entry: &bingo_sdk::catalogue::Entry,
        _: &bingo_sdk::HookContext,
        input: serde_json::Value,
        next: bingo_sdk::Next<'_, serde_json::Value, serde_json::Value>,
    ) -> Result<serde_json::Value, bingo_sdk::KernelError> {
        self.0.lock().unwrap().push(entry.name.to_string());
        next.run(input).await
    }
}
struct Watching(std::sync::Arc<std::sync::Mutex<Vec<String>>>);
#[async_trait::async_trait]
impl bingo_sdk::Plugin for Watching {
    fn manifest(&self) -> &'static bingo_sdk::PluginManifest {
        static MANIFEST: bingo_sdk::PluginManifest = bingo_sdk::PluginManifest {
            id: "test.watch",
            version: "0",
            provides: &[],
            requires: &[],
            uses: &[],
            config: None,
        };
        &MANIFEST
    }
    fn register(&self, registrar: &mut bingo_sdk::Registrar) -> Result<(), bingo_sdk::PluginError> {
        registrar.add(bingo_sdk::Contribution::Interceptor {
            placement: bingo_sdk::plugin::Placement::Outer,
            external: None,
            link: std::sync::Arc::new(FaceWatch(self.0.clone())),
        });
        Ok(())
    }
}
#[tokio::test]
async fn the_real_mcp_wire_face_raises_its_declared_event() {
    let directory = tempfile::tempdir().unwrap();
    let env = bingo_sdk::Env {
        home: directory.path().into(),
        config_dir: directory.path().join("config"),
        data_dir: directory.path().join("data"),
    };
    let seen = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
    let host = bingo_core::Host::build(
        vec![
            Box::new(bingo_mcp::McpPlugin::default()),
            Box::new(Watching(seen.clone())),
        ],
        bingo_core::HostConfig::new(env),
    )
    .await
    .unwrap();
    let face = host.service_wire(bingo_services::mcp::KEY.name()).unwrap();
    assert_eq!(
        face.call("servers", serde_json::Value::Null).await.unwrap(),
        json!({"servers":{}})
    );
    assert_eq!(*seen.lock().unwrap(), ["mcp.servers"]);
    assert!(
        host.open_service("second-owner", face)
            .unwrap_err()
            .message
            .contains("mcp.servers")
    );
    host.shutdown().await;
}
