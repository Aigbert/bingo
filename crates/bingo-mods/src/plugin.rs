//! The module executor contributes late plugins and interception links.
use crate::manager::Manager;
use async_trait::async_trait;
use bingo_sdk::*;
use serde_json::Value;
use std::{
    collections::BTreeMap,
    path::PathBuf,
    sync::{Arc, OnceLock},
};

#[derive(Clone, Default)]
pub struct ModsPlugin {
    directories: Vec<PathBuf>,
    settings: BTreeMap<String, Value>,
    project: Option<PathBuf>,
    manager: Arc<OnceLock<Arc<Manager>>>,
}
impl ModsPlugin {
    pub async fn reload(&self, root: &std::path::Path) -> Result<(), KernelError> {
        let manager = self.manager.get().ok_or_else(|| {
            KernelError::new(ErrorCode::NotReady, "module runtime has not started")
        })?;
        manager.reload_root(root).await
    }
    pub fn with_dirs(mut self, directories: Vec<PathBuf>) -> Self {
        self.directories = directories;
        self
    }
    pub fn with_settings(mut self, settings: BTreeMap<String, Value>) -> Self {
        self.settings = settings;
        self
    }
    pub fn with_project(mut self, project: PathBuf) -> Self {
        self.project = Some(project);
        self
    }
}
static MANIFEST: PluginManifest = PluginManifest {
    id: "bingo.mods",
    version: env!("CARGO_PKG_VERSION"),
    uses: &[],
    provides: &["service:bingo.plugin-host.callbacks"],
    requires: &[],
    config: Some(ConfigClaim {
        keys: &[("mods", Merge::ByName)],
        schema: || schemars::schema_for!(crate::config::Settings),
    }),
};
#[async_trait]
impl Plugin for ModsPlugin {
    fn manifest(&self) -> &'static PluginManifest {
        &MANIFEST
    }
    fn register(&self, registrar: &mut Registrar) -> Result<(), PluginError> {
        let manager = Arc::new(Manager::new(
            registrar.env().clone(),
            self.directories.clone(),
            self.settings.clone(),
            registrar.switched_off().clone(),
            registrar
                .config::<crate::config::Settings>()?
                .mods
                .limits()?,
        ));
        self.manager
            .set(manager.clone())
            .map_err(|_| PluginError::Failed("module runtime already registered".into()))?;
        registrar.add(Contribution::Plugins(manager.clone()));
        registrar.add(Contribution::service(
            &bingo_plugin_host::callback::KEY,
            manager.callbacks.clone(),
            None,
        ));
        registrar.add(Contribution::Tools(manager.clone()));
        registrar.add(Contribution::Commands(manager.clone()));
        registrar.add(Contribution::Interceptors(manager));
        Ok(())
    }
    async fn start(&self, host: HostHandle) -> Result<(), PluginError> {
        let cwd = self
            .project
            .clone()
            .map(Ok)
            .unwrap_or_else(std::env::current_dir)
            .map_err(|error| PluginError::Failed(error.to_string()))?;
        let floor = self
            .manager
            .get()
            .map(|manager| manager.every_floor())
            .unwrap_or(std::time::Duration::from_secs(1));
        let manager = self
            .manager
            .get()
            .ok_or_else(|| PluginError::Failed("module manager unavailable".into()))?;
        let dollar = crate::dollar::Runtime::install_in(&host, cwd.clone(), manager.env(), floor)
            .map_err(|error| PluginError::Failed(error.message))?;
        bingo_plugin_host::ops::install(&host)
            .map_err(|error| PluginError::Failed(error.message))?;
        if let Some(manager) = self.manager.get() {
            manager.start(&cwd, host, dollar).await;
        }
        Ok(())
    }
    async fn stop(&self) -> Result<(), PluginError> {
        if let Some(manager) = self.manager.get() {
            manager.stop().await;
        }
        Ok(())
    }
}
