//! Discovery supplies one shared source of module rows and links.
use crate::{engine::Limits, module::Module};
use async_trait::async_trait;
use bingo_plugin_host::{
    discovery,
    notice::{self, Notice, Notices},
};
use bingo_sdk::*;
use serde_json::Value;
use std::{
    collections::{BTreeMap, BTreeSet},
    path::{Path, PathBuf},
    sync::{Arc, Mutex, OnceLock, RwLock, Weak},
};

mod directories;
mod reload;
mod rescan;
mod switching;
pub(crate) struct Manager {
    pub(crate) callbacks: Arc<bingo_plugin_host::callback::Registry>,
    env: Env,
    directories: directories::Directories,
    settings: BTreeMap<String, Value>,
    switches: switching::State,
    limits: Limits,
    modules: RwLock<Vec<Arc<Module>>>,
    host: OnceLock<HostHandle>,
    dollar: OnceLock<Arc<crate::dollar::Runtime>>,
    this: OnceLock<Weak<Manager>>,
    watches: Mutex<BTreeMap<PathBuf, reload::Watch>>,
    reloading: tokio::sync::Mutex<()>,
    subscriptions: Mutex<BTreeSet<SessionId>>,
    notices: Arc<Notices>,
    stop: CancellationToken,
    tasks: Mutex<Vec<tokio::task::JoinHandle<()>>>,
}
impl Manager {
    pub fn new(
        env: Env,
        directories: Vec<PathBuf>,
        settings: BTreeMap<String, Value>,
        switched_off: BTreeSet<String>,
        limits: Limits,
    ) -> Self {
        Self {
            callbacks: Arc::default(),
            env,
            directories: directories::Directories::new(directories),
            settings,
            switches: switching::State {
                off: RwLock::new(switched_off),
                ..switching::State::default()
            },
            limits,
            modules: RwLock::new(Vec::new()),
            host: OnceLock::new(),
            dollar: OnceLock::new(),
            this: OnceLock::new(),
            watches: Mutex::new(BTreeMap::new()),
            reloading: tokio::sync::Mutex::new(()),
            subscriptions: Mutex::new(BTreeSet::new()),
            notices: Arc::new(Notices::default()),
            stop: CancellationToken::new(),
            tasks: Mutex::new(Vec::new()),
        }
    }
    pub async fn start(
        self: &Arc<Self>,
        cwd: &Path,
        host: HostHandle,
        dollar: Arc<crate::dollar::Runtime>,
    ) {
        let _ = self.directories.layers.set(discovery::dirs(&self.env, cwd));
        let _ = self.dollar.set(dollar);
        let _ = self.host.set(host.clone());
        let _ = self.this.set(Arc::downgrade(self));
        self.tasks
            .lock()
            .unwrap_or_else(|error| error.into_inner())
            .push(tokio::spawn(notice::drain(
                self.notices.clone(),
                host.clone(),
                self.stop.clone(),
            )));
        let found = self.directories.read(true, &self.notices, Some(&host));
        let mut modules = Vec::new();
        for found in found.into_iter().filter(|found| {
            bingo_plugin_host::runtime::runtime_of(&found.manifest, true)
                == Ok(PluginRuntime::Module)
        }) {
            let config = self
                .settings
                .get(&found.name)
                .cloned()
                .unwrap_or_else(|| serde_json::json!({}));
            let off = self
                .switches
                .off
                .read()
                .unwrap_or_else(|e| e.into_inner())
                .contains(&found.name);
            let module = Module::load(found, config, self.limits.clone(), &self.env, off).await;
            if !off && let Err(reason) = module.admission.decision() {
                self.notices.push(Notice::warn("PLUGIN_UNREADABLE", reason));
            }
            modules.push(module);
        }
        *self
            .modules
            .write()
            .unwrap_or_else(|error| error.into_inner()) = modules;
        for module in self.modules() {
            self.publish_callbacks(module);
        }
        self.write_types(&host).await;
        self.watch();
    }
    pub(crate) async fn write_types(&self, host: &HostHandle) {
        let result = async {
            let events = host
                .catalog(CatalogKind::Events)
                .await
                .map_err(|e| e.to_string())?;
            let tools = host
                .catalog(CatalogKind::Tools)
                .await
                .map_err(|e| e.to_string())?;
            for module in self
                .modules()
                .into_iter()
                .filter(|module| module.found.origin != PluginOrigin::Project)
            {
                if let Err(reason) = crate::types::write(&module.found.root, &events, &tools) {
                    self.notices.push(Notice::warn(
                        "MOD_TYPES",
                        format!("{}: {reason}", module.found.name),
                    ));
                }
            }
            Ok::<_, String>(())
        }
        .await;
        if let Err(reason) = result {
            self.notices.push(Notice::warn("MOD_TYPES", reason));
        }
    }
    pub(crate) async fn testing_call(
        &self,
        name: &str,
        method: &str,
        params: Value,
    ) -> Result<Value, String> {
        let module = self
            .modules()
            .into_iter()
            .find(|module| module.found.name == name)
            .ok_or("module is not loaded")?;
        if !module.admission.allowed() {
            return Err("module is not admitted".into());
        }
        bingo_plugin_host::ops::call(
            self.host.get().ok_or("host is not ready")?,
            name,
            &module.store,
            method,
            params,
        )
        .await
    }
    pub(crate) fn env(&self) -> &Env {
        &self.env
    }
    pub(crate) fn every_floor(&self) -> std::time::Duration {
        self.limits.every_time
    }
    pub(crate) fn publish_callbacks(&self, module: Arc<Module>) {
        if module.engine.is_none() {
            return;
        }
        self.callbacks.insert(
            module.found.name.clone(),
            Arc::new(crate::dollar::callback::Native {
                module,
                manager: self.this.get().cloned().unwrap_or_default(),
            }),
        );
    }
    fn modules(&self) -> Vec<Arc<Module>> {
        self.modules
            .read()
            .unwrap_or_else(|error| error.into_inner())
            .clone()
    }
    pub async fn stop(&self) {
        self.stop.cancel();
        let tasks =
            std::mem::take(&mut *self.tasks.lock().unwrap_or_else(|error| error.into_inner()));
        for task in tasks {
            let _ = task.await;
        }
        for module in self.modules() {
            module.close().await;
        }
        let retired = std::mem::take(&mut *self.switches.retired.lock().await);
        for module in retired {
            module.close().await;
        }
        if let Some(dollar) = self.dollar.get() {
            dollar.stop().await;
        }
    }
}
#[async_trait]
impl PluginSource for Manager {
    async fn switch_kind(&self, name: &str) -> plugin::PluginSwitch {
        switching::module_kind(self, name)
    }
    async fn switch(&self, name: &str, enabled: bool) -> Result<plugin::PluginSwitch, KernelError> {
        switching::switch_module(self, name, enabled).await
    }
    async fn approval(&self, name: &str) -> Option<Approval> {
        let modules = self.modules();
        let module = modules.iter().find(|module| {
            module.found.name == name
                && module.found.origin == PluginOrigin::Project
                && matches!(module.admission.state(), PluginState::Waiting { .. })
        })?;
        bingo_plugin_host::discovery::request(&module.found)
    }
    fn id(&self) -> &str {
        "bingo.mods"
    }
    async fn reload(&self) -> Result<PluginReload, KernelError> {
        self.rescan(true).await
    }
    async fn plugins(&self) -> Vec<PluginStatus> {
        self.modules()
            .iter()
            .map(|module| {
                module
                    .admission
                    .status(&module.found, PluginRuntime::Module, "bingo.mods")
            })
            .collect()
    }
    async fn registrations(
        &self,
        trust: &BTreeMap<String, PluginTrust>,
    ) -> Vec<PluginRegistration> {
        self.modules()
            .iter()
            .filter_map(|module| {
                module.admission.claim(
                    &module.found,
                    PluginRuntime::Module,
                    trust.get(&module.found.name),
                )
            })
            .collect()
    }
    async fn registered(&self, name: &str, result: Result<PluginGrant, String>) {
        if let Some(module) = self.modules().iter().find(|module| {
            module.found.name == name
                && matches!(module.admission.state(), PluginState::Registering)
        }) {
            let denied = result.is_err();
            module.admission.finish(result);
            if let Some(testing) = self.dollar.get().and_then(|dollar| dollar.testing.as_ref()) {
                let observes = !denied
                    && crate::link::Link(module.clone())
                        .matcher()
                        .matches(catalogue::SESSION_START, &Value::Null);
                if let Some(started) = testing.track_start(name, observes) {
                    let _ = module.started.set(started);
                }
            }
            if denied {
                module.close().await;
            }
        }
    }
}
#[async_trait]
impl InterceptorSource for Manager {
    fn id(&self) -> &str {
        "bingo.mods"
    }
    async fn interceptors(&self) -> Vec<ExternalInterceptor> {
        let mut links = vec![ExternalInterceptor {
            plugin: "bingo.mods".into(),
            link: Arc::new(reload::Boundary(
                self.this.get().cloned().unwrap_or_default(),
            )),
        }];
        links.extend(
            self.modules()
                .iter()
                .filter(|module| module.admission.allowed())
                .map(|module| ExternalInterceptor {
                    plugin: module.found.name.clone(),
                    link: Arc::new(crate::link::Link(module.clone())),
                }),
        );
        links
    }
}

#[async_trait]
impl ToolSource for Manager {
    fn id(&self) -> &str {
        "bingo.mods"
    }
    async fn tools(&self) -> Vec<Arc<dyn Tool>> {
        self.dollar
            .get()
            .map(|dollar| dollar.tools())
            .unwrap_or_default()
    }
}
#[async_trait]
impl CommandSource for Manager {
    fn id(&self) -> &str {
        "bingo.mods"
    }
    async fn commands(&self, _: &Path) -> Vec<Arc<dyn Command>> {
        let mut commands = self
            .dollar
            .get()
            .map(|dollar| dollar.commands())
            .unwrap_or_default();
        commands.extend(
            self.modules()
                .iter()
                .filter(|module| module.admission.allowed())
                .map(|module| {
                    Arc::new(crate::handlers::Command(module.found.name.clone()))
                        as Arc<dyn Command>
                }),
        );
        commands
    }
}
