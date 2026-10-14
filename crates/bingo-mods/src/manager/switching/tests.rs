use super::*;
use serde_json::json;

async fn fixture() -> (tempfile::TempDir, Arc<Manager>, Arc<Module>) {
    let root = tempfile::tempdir().unwrap();
    let directory = root.path().join("fixture");
    std::fs::create_dir(&directory).unwrap();
    std::fs::write(
        directory.join("plugin.json"),
        r#"{"name":"fixture","version":"1","module":"main.ts","uses":[]}"#,
    )
    .unwrap();
    std::fs::write(
        directory.join("main.ts"),
        "export function register(on) { on('session.start', () => {}); }",
    )
    .unwrap();
    let found = discovery::read(&directory, &Notices::default()).unwrap();
    let env = Env::rooted(root.path());
    let module = Module::load(found, json!({}), Limits::default(), &env, false).await;
    assert!(module.engine.is_some());
    module.admission.finish(Ok(PluginGrant {
        tier: Tier::Rewrite,
        uses: Default::default(),
    }));
    let manager = Arc::new(Manager::new(
        env,
        vec![],
        Default::default(),
        Default::default(),
        Limits::default(),
    ));
    manager.modules.write().unwrap().push(module.clone());
    (root, manager, module)
}

#[tokio::test]
async fn trust_granted_tool_authority_pins_even_before_lazy_registration() {
    let (_root, manager, module) = fixture().await;
    module.admission.finish(Ok(PluginGrant {
        tier: Tier::Rewrite,
        uses: ["tool".into()].into(),
    }));
    assert_eq!(module_kind(&manager, "fixture"), next("tools"));
    module.close().await;
}
