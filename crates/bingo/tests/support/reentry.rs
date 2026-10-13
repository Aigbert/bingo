//! The unchanged module crosses real Node pipes and the kernel's face chain.
use bingo_core::{Host, HostConfig};
use bingo_plugin_rpc::PluginRpcPlugin;
use bingo_sdk::*;
use serde_json::{Value, json};
use std::{path::Path, sync::Arc};

struct Echo;
#[async_trait::async_trait]
impl WireService for Echo {
    fn face(&self) -> Option<Face> {
        Some(Face::new("fixture").method::<Value, Value>("echo", false))
    }
    async fn call(&self, _: &str, input: Value) -> Result<Value, ServiceError> {
        Ok(input)
    }
}

fn install(root: &Path) -> std::path::PathBuf {
    let workspace = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let plugin = root.join("fixture");
    std::fs::create_dir(&plugin).unwrap();
    super::copy(&workspace.join("sdk/typescript"), &root.join("sdk"));
    std::fs::copy(
        workspace.join("crates/bingo-mods/tests/mods/reentrant.ts"),
        plugin.join("register.ts"),
    )
    .unwrap();
    std::fs::write(plugin.join("main.ts"),
        "import { plugin } from '../sdk/src/plugin.ts'; await plugin(new URL('./register.ts', import.meta.url));").unwrap();
    std::fs::write(
        plugin.join("plugin.json"),
        json!({
            "name":"fixture", "version":"1", "uses":["fixture"],
            "entry":{"command":"node", "args":["${PLUGIN_ROOT}/main.ts"]}
        })
        .to_string(),
    )
    .unwrap();
    plugin
}

#[tokio::test]
async fn node_reentry_skips_only_the_calling_hook_across_the_real_bridge() {
    if !super::node_available() {
        return;
    }
    let home = tempfile::tempdir().unwrap();
    let plugin = install(home.path());
    let mut config = HostConfig::new(Env::rooted(home.path())).with_layer(
        "user",
        json!({
            "pluginDirs":[plugin], "pluginTrust":{"fixture":{"tier":"rewrite","uses":["fixture"]}}
        }),
    );
    config.layers[0] = config.layers[0].clone().user();
    let host = Host::build(
        vec![Box::new(
            PluginRpcPlugin::default().with_project(home.path().into()),
        )],
        config,
    )
    .await
    .unwrap();
    host.open_service("fixture", Arc::new(Echo)).unwrap();
    let echo = host.service_wire("fixture").unwrap();
    let result = echo.call("echo", json!({"value":1})).await.unwrap();
    assert_eq!(result, json!({"value":1,"other":1}));
    let second = echo.call("echo", json!({"value":2})).await.unwrap();
    assert_eq!(second, json!({"value":2,"other":2}));
    host.shutdown().await;
}
