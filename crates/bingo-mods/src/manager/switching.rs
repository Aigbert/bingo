//! Retiring a generation removes future dispatch without closing a turn's links.
use super::*;
use bingo_sdk::plugin::PluginSwitch;

#[derive(Default)]
pub(super) struct State {
    pub off: RwLock<BTreeSet<String>>,
    pub retired: tokio::sync::Mutex<Vec<Arc<Module>>>,
}
pub(super) fn module_kind(manager: &Manager, name: &str) -> PluginSwitch {
    manager
        .modules()
        .iter()
        .find(|module| {
            module
                .admission
                .status(&module.found, PluginRuntime::Module, "bingo.mods")
                .id
                == name
        })
        .map_or_else(
            || next("module unavailable"),
            |module| classify(manager, module),
        )
}
fn classify(manager: &Manager, module: &Arc<Module>) -> PluginSwitch {
    if manager
        .dollar
        .get()
        .is_some_and(|dollar| dollar.has_tools(&module.found.name))
        || module.found.manifest.uses.iter().any(|name| name == "tool")
        || module
            .admission
            .decision()
            .ok()
            .flatten()
            .is_some_and(|grant| grant.uses.contains("tool"))
    {
        return next("tools");
    }
    let matcher = crate::link::Link(module.clone()).matcher();
    if let Some(entry) = catalogue::ALL.iter().find(|entry| {
        entry.shapes_request
            && matcher
                .patterns
                .iter()
                .any(|pattern| pattern.matches(&entry.name))
    }) {
        return next(&entry.name);
    }
    PluginSwitch::Now
}
pub(super) async fn switch_module(
    manager: &Manager,
    name: &str,
    enabled: bool,
) -> Result<PluginSwitch, KernelError> {
    if enabled {
        wait_retired(manager, name.strip_prefix("plugins:").unwrap_or(name)).await;
    }
    let _guard = manager.reloading.lock().await;
    let old = manager
        .modules()
        .into_iter()
        .find(|module| {
            module
                .admission
                .status(&module.found, PluginRuntime::Module, "bingo.mods")
                .id
                == name
        })
        .ok_or_else(|| reload::failed("module unavailable"))?;
    let name = old.found.name.clone();
    let name = name.as_str();
    let kind = classify(manager, &old);
    if kind != PluginSwitch::Now {
        return Ok(kind);
    }
    let found = switch_found(manager, &old);
    let config = manager
        .settings
        .get(name)
        .cloned()
        .unwrap_or_else(|| serde_json::json!({}));
    let module = Module::load(
        found,
        config,
        manager.limits.clone(),
        &manager.env,
        !enabled,
    )
    .await;
    // A previously unloaded module has no hook metadata until register has run.
    let kind = classify(manager, &module);
    if enabled && kind != PluginSwitch::Now {
        module.close().await;
        return Ok(kind);
    }
    activate_switch(manager, old, module, enabled).await?;
    Ok(PluginSwitch::Now)
}
async fn activate_switch(
    manager: &Manager,
    old: Arc<Module>,
    module: Arc<Module>,
    enabled: bool,
) -> Result<(), KernelError> {
    let name = module.found.name.as_str();
    {
        let mut off = manager
            .switches
            .off
            .write()
            .unwrap_or_else(|e| e.into_inner());
        if enabled {
            off.remove(name);
        } else {
            off.insert(name.into());
        }
    }
    let owned = old.engine.is_some();
    if owned {
        manager.callbacks.remove(name);
        if let Some(dollar) = manager.dollar.get() {
            dollar.unregister(name);
        }
    }
    manager.replace(&old, module.clone());
    if owned {
        manager.switches.retired.lock().await.push(old);
    }
    manager.publish_callbacks(module.clone());
    let host = manager
        .host
        .get()
        .ok_or_else(|| reload::failed("module host unavailable"))?;
    host.catalog(CatalogKind::Plugins).await?;
    module.admission.decision().map_err(reload::failed)?;
    if !enabled && owned {
        manager.clear_signals(name).await;
    }
    manager.restart_sessions(&module, host).await?;
    manager.baseline(false);
    Ok(())
}
fn switch_found(manager: &Manager, old: &Module) -> discovery::Found {
    let mut found = old.found.clone();
    if found.origin == PluginOrigin::Project {
        let authority = manager.host.get().and_then(|host| host.service(&APPROVALS));
        found.state = discovery::project_state(&found, authority.as_deref());
    }
    found
}
async fn wait_retired(manager: &Manager, name: &str) {
    loop {
        reap(manager).await;
        if !manager
            .switches
            .retired
            .lock()
            .await
            .iter()
            .any(|module| module.found.name == name)
        {
            return;
        }
        tokio::time::sleep(std::time::Duration::from_millis(10)).await;
    }
}
pub(super) async fn reap(manager: &Manager) {
    let _guard = manager.reloading.lock().await;
    let ready: Vec<_> = manager
        .switches
        .retired
        .lock()
        .await
        .iter()
        .filter(|module| Arc::strong_count(module) == 1)
        .cloned()
        .collect();
    for module in ready {
        module.close().await;
        manager.clear_signals(&module.found.name).await;
        if let Some(dollar) = manager.dollar.get() {
            dollar.reload(&module.found.name).await;
        }
        // Keep closing generations visible to enable until cleanup finishes.
        manager
            .switches
            .retired
            .lock()
            .await
            .retain(|entry| !Arc::ptr_eq(entry, &module));
    }
}
fn next(why: &str) -> PluginSwitch {
    PluginSwitch::NextStart { why: why.into() }
}

#[cfg(test)]
mod tests;
