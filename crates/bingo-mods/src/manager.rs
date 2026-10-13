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

mod reload;
pub(crate) struct Manager {
    pub(crate) callbacks: Arc<bingo_plugin_host::callback::Registry>,
    env: Env,
    directories: Vec<PathBuf>,
    settings: BTreeMap<String, Value>,
    switched_off: BTreeSet<String>,
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
            directories,
            settings,
            switched_off,
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
        let _ = self.dollar.set(dollar);
        let _ = self.host.set(host.clone());
        let _ = self.this.set(Arc::downgrade(self));
        self.tasks
            .lock()
            .unwrap_or_else(|error| error.into_inner())
            .push(tokio::spawn(notice::drain(
                self.notices.clone(),
                host,
                self.stop.clone(),
            )));
        let found = discovery::with_explicit(
            &discovery::dirs(&self.env, cwd),
            &self.directories,
            &self.notices,
        );
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
            let off = self.switched_off.contains(&found.name);
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
        self.watch();
    }
    pub(crate) fn publish_callbacks(&self, module: Arc<Module>) {
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
        if let Some(dollar) = self.dollar.get() {
            dollar.stop().await;
        }
    }
}
#[async_trait]
impl PluginSource for Manager {
    fn id(&self) -> &str {
        "bingo.mods"
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
        if let Some(module) = self
            .modules()
            .iter()
            .find(|module| module.found.name == name)
        {
            let denied = result.is_err();
            module.admission.finish(result);
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
        self.dollar
            .get()
            .map(|dollar| dollar.commands())
            .unwrap_or_default()
    }
}
