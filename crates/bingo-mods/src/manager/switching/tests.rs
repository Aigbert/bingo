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

#[tokio::test]
async fn cancelling_the_switch_waiter_does_not_lose_the_retiring_generation() {
    let (_root, manager, old) = fixture().await;
    let host = bingo_sdk::testing::TestHost::default();
    manager.host.set(host.handle()).unwrap();
    manager.this.set(Arc::downgrade(&manager)).unwrap();
    let retired = manager.switches.retired.lock().await;
    let caller = {
        let manager = manager.clone();
        tokio::spawn(async move { manager.switch("fixture", false).await })
    };
    tokio::time::timeout(std::time::Duration::from_secs(10), async {
        while manager.modules()[0].engine.is_some() {
            tokio::task::yield_now().await;
        }
    })
    .await
    .unwrap();
    caller.abort();
    let _ = caller.await;
    drop(retired);
    let _completed = manager.reloading.lock().await;
    assert!(
        manager
            .switches
            .retired
            .lock()
            .await
            .iter()
            .any(|entry| Arc::ptr_eq(entry, &old))
    );
    old.close().await;
}

#[tokio::test]
async fn retirement_does_not_hold_the_list_while_awaiting_host_cleanup() {
    let (_root, manager, module) = fixture().await;
    manager.modules.write().unwrap().clear();
    manager.switches.retired.lock().await.push(module);
    let host = bingo_sdk::testing::TestHost::default();
    let entered = Arc::new(tokio::sync::Notify::new());
    let release = Arc::new(tokio::sync::Notify::new());
    host.on_sessions({
        let entered = entered.clone();
        let release = release.clone();
        move |_| {
            let entered = entered.clone();
            let release = release.clone();
            async move {
                entered.notify_one();
                release.notified().await;
                Ok(vec![])
            }
        }
    });
    manager.host.set(host.handle()).unwrap();
    let cleanup = {
        let manager = manager.clone();
        tokio::spawn(async move { reap(&manager).await })
    };
    entered.notified().await;
    let available = manager.switches.retired.try_lock().is_ok();
    release.notify_one();
    cleanup.await.unwrap();
    assert!(available, "retired list locked across host I/O");
}

#[tokio::test]
async fn enabling_a_held_generation_replies_without_holding_other_switches() {
    let (_root, manager, old) = fixture().await;
    let host = bingo_sdk::testing::TestHost::default();
    host.on_catalog(|kind| async move {
        Ok(Catalog {
            kind,
            entries: vec![],
        })
    });
    manager.host.set(host.handle()).unwrap();
    manager.this.set(Arc::downgrade(&manager)).unwrap();
    manager.switch("fixture", false).await.unwrap();
    tokio::time::pause();
    let result = tokio::time::timeout(
        std::time::Duration::from_secs(6),
        manager.switch("fixture", true),
    )
    .await;
    assert!(result.is_ok(), "enable held the command indefinitely");
    let result = result.unwrap().unwrap();
    assert!(format!("{result:?}").contains("AfterTurn"));
    assert!(matches!(
        manager.plugins().await[0].state,
        PluginState::Pending { now: false, .. }
    ));
    manager.switch("fixture", false).await.unwrap();
    drop(old);
    reap(&manager).await;
    // The later wish must win when the queued enable can finally proceed.
    tokio::time::advance(std::time::Duration::from_secs(1)).await;
    assert!(manager.modules()[0].engine.is_none());
}

fn runtime(manager: &Arc<Manager>) -> Arc<crate::dollar::Runtime> {
    let host = bingo_sdk::testing::TestHost::default();
    host.on_catalog(|kind| async move {
        Ok(Catalog {
            kind,
            entries: vec![],
        })
    });
    let runtime = crate::dollar::Runtime::install(
        &host.handle(),
        manager.env.home.clone(),
        std::time::Duration::from_secs(1),
    )
    .unwrap();
    manager.host.set(host.handle()).unwrap();
    manager.this.set(Arc::downgrade(manager)).unwrap();
    assert!(manager.dollar.set(runtime.clone()).is_ok());
    runtime
}
fn process(runtime: &crate::dollar::Runtime, module: &Module) -> (String, SessionId) {
    let session = SessionId::mint();
    let input = serde_json::from_value(json!({"session":session,"argv":[
        bingo_process::shell::resolve().unwrap().path(), "-c", "cat"]}))
    .unwrap();
    let id = runtime
        .processes_for_test()
        .spawn(
            "fixture",
            &input,
            &module.found.root,
            Default::default(),
            module.engine.as_ref().unwrap().stopped(),
        )
        .unwrap();
    (id, session)
}

#[tokio::test]
async fn reloading_an_off_module_preserves_the_retired_generations_process() {
    for rescan in [false, true] {
        let (_root, manager, old) = fixture().await;
        let runtime = runtime(&manager);
        let (id, session) = process(&runtime, &old);
        manager.switch("fixture", false).await.unwrap();
        if rescan {
            std::fs::write(
                old.found.root.join("main.ts"),
                "export function register(on) {} // changed",
            )
            .unwrap();
            manager.rescan(true).await.unwrap();
        } else {
            manager.reload_root(&old.found.root).await.unwrap();
        }
        let alive = runtime
            .processes_for_test()
            .write("fixture", &session, &id, Some("still alive\n".into()))
            .await;
        drop(old);
        reap(&manager).await;
        assert!(
            alive.is_ok(),
            "reload destroyed a running generation: {alive:?}"
        );
    }
}
