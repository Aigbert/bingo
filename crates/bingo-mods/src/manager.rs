//! Discovery supplies one shared source of module rows and links.
use crate::{engine::Limits, module::Module};
use bingo_plugin_host::{discovery, manifest::Launch, notice::Notices};
use bingo_sdk::*;
use async_trait::async_trait;
use serde_json::Value;
use std::{collections::{BTreeMap, BTreeSet}, path::{Path, PathBuf}, sync::{Arc, OnceLock}};

pub(crate) struct Manager {
    env: Env,
    directories: Vec<PathBuf>,
    settings: BTreeMap<String, Value>,
    switched_off: BTreeSet<String>,
    limits: Limits,
    modules: OnceLock<Vec<Arc<Module>>>,
}
impl Manager {
    pub fn new(env: Env, directories: Vec<PathBuf>, settings: BTreeMap<String, Value>, switched_off: BTreeSet<String>, limits: Limits) -> Self {
        Self { env, directories, settings, switched_off, limits, modules: OnceLock::new() }
    }
    pub async fn start(&self, cwd: &Path) {
        let notices = Notices::default();
        let found = discovery::with_explicit(&discovery::dirs(&self.env, cwd), &self.directories, &notices);
        let mut modules = Vec::new();
        for found in found.into_iter().filter(|found| matches!(found.manifest.launch, Launch::Module(_))) {
            let config = self.settings.get(&found.name).cloned().unwrap_or(Value::Null);
            let off = self.switched_off.contains(&found.name);
            modules.push(Module::load(found, config, self.limits.clone(), &self.env, off).await);
        }
        let _ = self.modules.set(modules);
    }
    fn modules(&self) -> &[Arc<Module>] { self.modules.get().map(Vec::as_slice).unwrap_or_default() }
    pub async fn stop(&self) { for module in self.modules() { module.close().await; } }
}
#[async_trait]
impl PluginSource for Manager {
    fn id(&self) -> &str { "bingo.mods" }
    async fn plugins(&self) -> Vec<PluginStatus> {
        self.modules().iter().map(|module| module.admission.status(&module.found, PluginRuntime::Module, "bingo.mods")).collect()
    }
    async fn registrations(&self, trust: &BTreeMap<String, PluginTrust>) -> Vec<PluginRegistration> {
        self.modules().iter().filter_map(|module| module.admission.claim(&module.found, PluginRuntime::Module, trust.get(&module.found.name))).collect()
    }
    async fn registered(&self, name: &str, result: Result<PluginGrant, String>) {
        if let Some(module) = self.modules().iter().find(|module| module.found.name == name) {
            let denied = result.is_err();
            module.admission.finish(result);
            if denied { module.close().await; }
        }
    }
}
#[async_trait]
impl InterceptorSource for Manager {
    fn id(&self) -> &str { "bingo.mods" }
    async fn interceptors(&self) -> Vec<ExternalInterceptor> {
        Vec::new()
    }
}
