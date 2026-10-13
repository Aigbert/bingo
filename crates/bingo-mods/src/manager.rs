//! Discovery supplies one shared source of module rows and links.
use crate::{engine::Limits, module::Module};
use async_trait::async_trait;
use bingo_plugin_host::{
    discovery,
    manifest::Launch,
    notice::{self, Notice, Notices},
};
use bingo_sdk::*;
use serde_json::Value;
use std::{
    collections::{BTreeMap, BTreeSet},
    path::{Path, PathBuf},
    sync::{Arc, Mutex, OnceLock},
};

pub(crate) struct Manager {
    env: Env,
    directories: Vec<PathBuf>,
    settings: BTreeMap<String, Value>,
    switched_off: BTreeSet<String>,
    limits: Limits,
    modules: OnceLock<Vec<Arc<Module>>>,
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
            env,
            directories,
            settings,
            switched_off,
            limits,
            modules: OnceLock::new(),
            notices: Arc::new(Notices::default()),
            stop: CancellationToken::new(),
            tasks: Mutex::new(Vec::new()),
        }
    }
    pub async fn start(&self, cwd: &Path, host: HostHandle) {
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
        for found in found
            .into_iter()
            .filter(|found| matches!(found.manifest.launch, Launch::Module(_)))
        {
            let config = self
                .settings
                .get(&found.name)
                .cloned()
                .unwrap_or(Value::Null);
            let off = self.switched_off.contains(&found.name);
            let module = Module::load(found, config, self.limits.clone(), &self.env, off).await;
            if !off && let Err(reason) = module.admission.decision() {
                self.notices.push(Notice::warn("PLUGIN_UNREADABLE", reason));
            }
            modules.push(module);
        }
        let _ = self.modules.set(modules);
    }
    fn modules(&self) -> &[Arc<Module>] {
        self.modules.get().map(Vec::as_slice).unwrap_or_default()
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
        self.modules()
            .iter()
            .filter(|module| module.admission.allowed())
            .map(|module| ExternalInterceptor {
                plugin: module.found.name.clone(),
                link: Arc::new(crate::link::Link(module.clone())),
            })
            .collect()
    }
}
