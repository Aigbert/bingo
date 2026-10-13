use bingo_core::{Host, HostConfig};
use bingo_mods::ModsPlugin;
use bingo_sdk::{CatalogKind, Env, HostApi};
use serde_json::json;

#[tokio::test]
async fn an_explicit_module_is_listed_with_its_granted_runtime() {
    let home = tempfile::tempdir().unwrap();
    let root = home.path().join("fixture");
    std::fs::create_dir(&root).unwrap();
    std::fs::write(root.join("plugin.json"), json!({"name":"fixture", "version":"1", "module":"main.ts", "uses":["state"]}).to_string()).unwrap();
    std::fs::write(root.join("main.ts"), "export function register(on) { on('tool.call', () => ({parts:[]})); }").unwrap();
    let host = Host::build(vec![Box::new(ModsPlugin::default().with_dirs(vec![root]))], HostConfig::new(Env::rooted(home.path()))).await.unwrap();
    let catalog = host.catalog(CatalogKind::Plugins).await.unwrap();
    let fixture = catalog.entries.iter().find(|entry| entry.id == "fixture").expect("module is discovered");
    assert_eq!(fixture.meta["runtime"], "module");
    assert_eq!(fixture.meta["tier"], "rewrite");
    assert_eq!(fixture.meta["uses"], json!(["state"]));
    assert_eq!(fixture.meta["enabled"], true);
    host.shutdown().await;
}
