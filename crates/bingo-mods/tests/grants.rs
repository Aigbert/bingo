use bingo_core::{Host, HostConfig};
use bingo_mods::ModsPlugin;
use bingo_sdk::{CatalogKind, Env, HostApi};
use serde_json::{Value, json};

fn fixture(root: &std::path::Path, name: &str, source: &str, uses: &[&str]) {
    std::fs::create_dir_all(root).unwrap();
    std::fs::write(
        root.join("plugin.json"),
        json!({
            "name":name, "version":"1", "module":"main.ts", "uses":uses
        })
        .to_string(),
    )
    .unwrap();
    std::fs::write(root.join("main.ts"), source).unwrap();
}

#[tokio::test]
async fn registration_can_deny_a_module_with_a_named_reason() {
    let home = tempfile::tempdir().unwrap();
    let policy = home.path().join("a-policy");
    let target = home.path().join("target");
    fixture(
        &policy,
        "a-policy",
        "export function register(on) { on('plugin.register', {name:'target'}, () => ({reason:'disabled by local policy'})); }",
        &[],
    );
    fixture(&target, "target", "export function register(on) {}", &[]);
    let host = Host::build(
        vec![Box::new(
            ModsPlugin::default().with_dirs(vec![policy, target]),
        )],
        HostConfig::new(Env::rooted(home.path())),
    )
    .await
    .unwrap();
    let catalog = host.catalog(CatalogKind::Plugins).await.unwrap();
    let target = catalog
        .entries
        .iter()
        .find(|row| row.id == "target")
        .unwrap();
    assert_eq!(target.meta["enabled"], false, "{target:?}");
    let reason = target.meta["reason"].as_str().unwrap();
    assert!(
        reason.contains("target") && reason.contains("disabled by local policy"),
        "{reason}"
    );
    host.shutdown().await;
}

struct Echo;
#[async_trait::async_trait]
impl bingo_sdk::WireService for Echo {
    fn face(&self) -> Option<bingo_sdk::Face> {
        Some(bingo_sdk::Face::new("fixture").method::<Value, Value>("echo", false))
    }
    async fn call(&self, _: &str, input: Value) -> Result<Value, bingo_sdk::ServiceError> {
        Ok(input)
    }
}

#[tokio::test]
async fn a_project_model_call_requires_the_users_grant() {
    let home = tempfile::tempdir().unwrap();
    let project = tempfile::tempdir().unwrap();
    fixture(
        &project.path().join(".bingo/plugins/target"),
        "target",
        r#"
      export function register(on) {
        on('fixture.echo', async ($, e) => {
          try { await $.model.complete({model:'offline', maxTokens:1, system:[], messages:[], tools:[]}); }
          catch(error) { await $.state.set('reason', String(error)); }
        });
      }
    "#,
        &["model", "state"],
    );
    for trusted in [false, true] {
        let mut config = HostConfig::new(Env::rooted(home.path()));
        if trusted {
            config = config.with_layer(
                "user",
                json!({"pluginTrust":{"target":{"tier":"rewrite", "uses":["model"]}}}),
            );
            let user = config.layers.pop().unwrap().user();
            config.layers.push(user);
        }
        let host = Host::build(
            vec![Box::new(
                ModsPlugin::default().with_project(project.path().into()),
            )],
            config,
        )
        .await
        .unwrap();
        host.open_service("fixture", std::sync::Arc::new(Echo))
            .unwrap();
        host.service_wire("fixture")
            .unwrap()
            .call("echo", json!({}))
            .await
            .unwrap();
        let store =
            bingo_plugin_host::state::Store::new(&Env::rooted(home.path()).data_dir, "target");
        let value = store
            .get("host".into(), "reason".into())
            .await
            .unwrap()
            .unwrap()
            .value;
        let reason = value.as_str().unwrap();
        if trusted {
            assert!(!reason.contains("model grant"), "{reason}");
            assert!(
                reason.contains("SESSION_NOT_FOUND"),
                "the granted request reaches the offline host: {reason}"
            );
        } else {
            assert!(reason.contains("model grant"), "{reason}");
        }
        host.shutdown().await;
    }
}

#[tokio::test]
async fn module_options_are_validated_before_register_runs() {
    let home = tempfile::tempdir().unwrap();
    let root = home.path().join("target");
    fixture(&root, "target", "export function register(on, options) {}", &[]);
    let path = root.join("plugin.json");
    let mut manifest: Value = serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
    manifest["config"] = json!({"type":"object", "properties":{"label":{"type":"string"}}, "required":["label"], "additionalProperties":false});
    std::fs::write(path, manifest.to_string()).unwrap();
    let plugin = ModsPlugin::default().with_dirs(vec![root]).with_settings([("target".into(), json!({"label":42}))].into());
    let host = Host::build(vec![Box::new(plugin)], HostConfig::new(Env::rooted(home.path()))).await.unwrap();
    let catalog = host.catalog(CatalogKind::Plugins).await.unwrap();
    let row = catalog.entries.iter().find(|row| row.id == "target").unwrap();
    assert_eq!(row.meta["enabled"], false);
    assert!(row.meta["reason"].as_str().unwrap().contains("label"), "{row:?}");
    host.shutdown().await;
}
