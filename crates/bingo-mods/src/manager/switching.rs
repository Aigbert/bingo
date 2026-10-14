//! Retiring a generation removes future dispatch without closing a turn's links.
use super::*;
use bingo_sdk::plugin::PluginSwitch;

impl Manager {
    pub(super) fn module_kind(&self, name: &str) -> PluginSwitch {
        self.modules()
            .iter()
            .find(|module| {
                module
                    .admission
                    .status(&module.found, PluginRuntime::Module, "bingo.mods")
                    .id
                    == name
            })
            .map_or_else(|| next("module unavailable"), |module| self.kind(module))
    }
    fn kind(&self, module: &Arc<Module>) -> PluginSwitch {
        if self
            .dollar
            .get()
            .is_some_and(|dollar| dollar.has_tools(&module.found.name))
            || module.found.manifest.uses.iter().any(|name| name == "tool")
        {
            return next("tools");
        }
        let matcher = crate::link::Link(module.clone()).matcher();
        if catalogue::ALL.iter().any(|entry| {
            entry.shapes_request
                && matcher
                    .patterns
                    .iter()
                    .any(|pattern| pattern.matches(&entry.name))
        }) {
            return next("prompt.compose");
        }
        PluginSwitch::Now
    }
    pub(super) async fn switch_module(
        &self,
        name: &str,
        enabled: bool,
    ) -> Result<PluginSwitch, KernelError> {
        if enabled {
            self.wait_retired(name.strip_prefix("plugins:").unwrap_or(name))
                .await;
        }
        let _guard = self.reloading.lock().await;
        let old = self
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
        let kind = self.kind(&old);
        if kind != PluginSwitch::Now {
            return Ok(kind);
        }
        let found = self.switch_found(&old);
        let config = self
            .settings
            .get(name)
            .cloned()
            .unwrap_or_else(|| serde_json::json!({}));
        let module = Module::load(found, config, self.limits.clone(), &self.env, !enabled).await;
        // A previously unloaded module has no hook metadata until register has run.
        let kind = self.kind(&module);
        if enabled && kind != PluginSwitch::Now {
            module.close().await;
            return Ok(kind);
        }
        self.activate_switch(old, module, enabled).await?;
        Ok(PluginSwitch::Now)
    }
    async fn activate_switch(
        &self,
        old: Arc<Module>,
        module: Arc<Module>,
        enabled: bool,
    ) -> Result<(), KernelError> {
        let name = module.found.name.as_str();
        {
            let mut off = self.switched_off.write().unwrap_or_else(|e| e.into_inner());
            if enabled {
                off.remove(name);
            } else {
                off.insert(name.into());
            }
        }
        let owned = old.engine.is_some();
        if owned {
            self.callbacks.remove(name);
            if let Some(dollar) = self.dollar.get() {
                dollar.unregister(name);
            }
        }
        self.replace(&old, module.clone());
        if owned {
            self.retired.lock().await.push(old);
        }
        self.publish_callbacks(module.clone());
        let host = self
            .host
            .get()
            .ok_or_else(|| reload::failed("module host unavailable"))?;
        host.catalog(CatalogKind::Plugins).await?;
        module.admission.decision().map_err(reload::failed)?;
        if !enabled && owned {
            self.clear_signals(name).await;
        }
        self.restart_sessions(&module, host).await?;
        self.baseline(false);
        Ok(())
    }
    fn switch_found(&self, old: &Module) -> discovery::Found {
        let mut found = old.found.clone();
        if found.origin == PluginOrigin::Project {
            let authority = self.host.get().and_then(|host| host.service(&APPROVALS));
            found.state = discovery::project_state(&found, authority.as_deref());
        }
        found
    }
    async fn wait_retired(&self, name: &str) {
        loop {
            self.reap().await;
            if !self
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
    pub(super) async fn reap(&self) {
        let mut retired = self.retired.lock().await;
        let mut i = 0;
        while i < retired.len() {
            if Arc::strong_count(&retired[i]) != 1 {
                i += 1;
                continue;
            }
            let module = retired.remove(i);
            module.close().await;
            self.clear_signals(&module.found.name).await;
            if let Some(dollar) = self.dollar.get() {
                dollar.reload(&module.found.name).await;
            }
        }
    }
}
fn next(why: &str) -> PluginSwitch {
    PluginSwitch::NextStart { why: why.into() }
}
