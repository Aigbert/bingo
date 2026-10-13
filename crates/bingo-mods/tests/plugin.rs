use bingo_core::{Host, HostConfig};
use bingo_mods::ModsPlugin;
use bingo_sdk::{CatalogKind, Env, HostApi};
use serde_json::json;

#[tokio::test]
async fn an_explicit_module_is_listed_with_its_granted_runtime() {
    let home = tempfile::tempdir().unwrap();
    let root = home.path().join("fixture");
    std::fs::create_dir(&root).unwrap();
    std::fs::write(
        root.join("plugin.json"),
        json!({"name":"fixture", "version":"1", "module":"main.ts", "uses":["state"]}).to_string(),
    )
    .unwrap();
    std::fs::write(
        root.join("main.ts"),
        "export function register(on) { on('tool.call', () => ({parts:[]})); }",
    )
    .unwrap();
    let host = Host::build(
        vec![Box::new(ModsPlugin::default().with_dirs(vec![root]))],
        HostConfig::new(Env::rooted(home.path())),
    )
    .await
    .unwrap();
    let catalog = host.catalog(CatalogKind::Plugins).await.unwrap();
    let fixture = catalog
        .entries
        .iter()
        .find(|entry| entry.id == "fixture")
        .expect("module is discovered");
    assert_eq!(fixture.meta["runtime"], "module");
    assert_eq!(fixture.meta["tier"], "rewrite");
    assert_eq!(fixture.meta["uses"], json!(["state"]));
    assert_eq!(fixture.meta["enabled"], true);
    host.shutdown().await;
}

struct Echo;
#[async_trait::async_trait]
impl bingo_sdk::WireService for Echo {
    fn face(&self) -> Option<bingo_sdk::Face> {
        Some(
            bingo_sdk::Face::new("fixture")
                .method::<serde_json::Value, serde_json::Value>("echo", false),
        )
    }
    async fn call(
        &self,
        _: &str,
        input: serde_json::Value,
    ) -> Result<serde_json::Value, bingo_sdk::ServiceError> {
        Ok(input)
    }
}

#[tokio::test]
async fn a_module_wraps_a_service_on_the_real_host_chain() {
    let home = tempfile::tempdir().unwrap();
    let root = home.path().join("fixture");
    std::fs::create_dir(&root).unwrap();
    std::fs::write(
        root.join("plugin.json"),
        json!({"name":"fixture", "version":"1", "module":"main.ts"}).to_string(),
    )
    .unwrap();
    std::fs::write(root.join("main.ts"), "export function register(on) { on('fixture.echo', async ($, e, next) => ({...await next(e), wrapped:true})); }").unwrap();
    let host = Host::build(
        vec![Box::new(ModsPlugin::default().with_dirs(vec![root]))],
        HostConfig::new(Env::rooted(home.path())),
    )
    .await
    .unwrap();
    host.open_service("fixture", std::sync::Arc::new(Echo))
        .unwrap();
    let result = host
        .service_wire("fixture")
        .unwrap()
        .call("echo", json!({"value":1}))
        .await
        .unwrap();
    assert_eq!(result, json!({"value":1, "wrapped":true}));
    host.shutdown().await;
}

#[tokio::test]
async fn a_catalogue_hook_cannot_mint_grants() {
    let home = tempfile::tempdir().unwrap();
    let root = home.path().join("fixture");
    std::fs::create_dir(&root).unwrap();
    std::fs::write(
        root.join("plugin.json"),
        json!({"name":"fixture", "version":"1", "module":"main.ts", "uses":["state"]}).to_string(),
    )
    .unwrap();
    std::fs::write(root.join("main.ts"), "export function register(on) { on('prompt.autocomplete', async ($, e, next) => { const result=await next(e); return {...result, entries:result.entries.map(row => row.id==='fixture' ? {...row, meta:{...row.meta, tier:'full', uses:['model']}} : row)}; }); }").unwrap();
    let host = Host::build(
        vec![Box::new(ModsPlugin::default().with_dirs(vec![root]))],
        HostConfig::new(Env::rooted(home.path())),
    )
    .await
    .unwrap();
    let catalog = host.catalog(CatalogKind::Plugins).await.unwrap();
    let fixture = catalog
        .entries
        .iter()
        .find(|entry| entry.id == "fixture")
        .unwrap();
    assert_eq!(fixture.meta["tier"], "rewrite");
    assert_eq!(fixture.meta["uses"], json!(["state"]));
    host.shutdown().await;
}

#[tokio::test]
async fn dollar_state_is_granted_and_each_operation_is_raised_once() {
    let home = tempfile::tempdir().unwrap();
    let root = home.path().join("fixture");
    std::fs::create_dir(&root).unwrap();
    std::fs::write(
        root.join("plugin.json"),
        json!({"name":"fixture", "version":"1", "module":"main.ts", "uses":["state"]}).to_string(),
    )
    .unwrap();
    std::fs::write(root.join("main.ts"), r#"
      export function register(on) {
        let writes = 0;
        on('fixture.echo', async ($, e) => {
          await $.state.set('key', e);
          return {...await $.state.get('key'), writes};
        });
        on('state.set', async ($, e, next) => { writes++; return next({...e, value:{...e.value, seen:true}}); });
      }
    "#).unwrap();
    let host = Host::build(
        vec![Box::new(ModsPlugin::default().with_dirs(vec![root]))],
        HostConfig::new(Env::rooted(home.path())),
    )
    .await
    .unwrap();
    host.open_service("fixture", std::sync::Arc::new(Echo))
        .unwrap();
    let result = host
        .service_wire("fixture")
        .unwrap()
        .call("echo", json!({"value":1}))
        .await
        .unwrap();
    assert_eq!(
        result,
        json!({"version":1, "value":{"value":1,"seen":true}, "writes":1})
    );
    host.shutdown().await;
}

#[tokio::test]
async fn an_undeclared_state_call_is_refused_in_words() {
    let home = tempfile::tempdir().unwrap();
    let root = home.path().join("fixture");
    std::fs::create_dir(&root).unwrap();
    std::fs::write(
        root.join("plugin.json"),
        json!({"name":"fixture", "version":"1", "module":"main.ts"}).to_string(),
    )
    .unwrap();
    std::fs::write(root.join("main.ts"), "export function register(on) { on('fixture.echo', async ($) => { try { await $.state.get('key'); return {error:'not refused'}; } catch(error) { return {error:String(error)}; } }); }").unwrap();
    let host = Host::build(
        vec![Box::new(ModsPlugin::default().with_dirs(vec![root]))],
        HostConfig::new(Env::rooted(home.path())),
    )
    .await
    .unwrap();
    host.open_service("fixture", std::sync::Arc::new(Echo))
        .unwrap();
    let result = host
        .service_wire("fixture")
        .unwrap()
        .call("echo", json!({}))
        .await
        .unwrap();
    assert!(
        result["error"].as_str().unwrap().contains("state grant"),
        "{result}"
    );
    host.shutdown().await;
}

#[tokio::test]
async fn a_timed_out_inner_hook_gets_a_fresh_catch_budget() {
    let home = tempfile::tempdir().unwrap();
    let root = home.path().join("fixture");
    std::fs::create_dir(&root).unwrap();
    std::fs::write(
        root.join("plugin.json"),
        json!({"name":"fixture", "version":"1", "module":"main.ts"}).to_string(),
    )
    .unwrap();
    std::fs::write(
        root.join("main.ts"),
        r#"
      export function register(on) {
        on('fixture.echo', async ($, e, next) => ({...await next(e), outer:true}));
        on('fixture.echo', {test:1}, () => { while(true) {} }).catch(($, e, next) => ({
          kind:next.error.kind, called:next.called, budget:next.budget.ms
        }));
      }
    "#,
    )
    .unwrap();
    let config =
        HostConfig::new(Env::rooted(home.path())).with_layer("test", json!({"mods":{"hookMs":50}}));
    let host = Host::build(
        vec![Box::new(ModsPlugin::default().with_dirs(vec![root]))],
        config,
    )
    .await
    .unwrap();
    host.open_service("fixture", std::sync::Arc::new(Echo))
        .unwrap();
    let result = host
        .service_wire("fixture")
        .unwrap()
        .call("echo", json!({"test":1}))
        .await
        .unwrap();
    assert_eq!(
        result,
        json!({"kind":"timeout", "called":false, "budget":1000, "outer":true})
    );
    host.shutdown().await;
}
