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
    provides: &[],
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
        registrar.add(Contribution::Interceptors(manager));
        Ok(())
    }
    async fn start(&self, host: HostHandle) -> Result<(), PluginError> {
        bingo_plugin_host::ops::install(&host)
            .map_err(|error| PluginError::Failed(error.message))?;
        let cwd = self
            .project
            .clone()
            .map(Ok)
            .unwrap_or_else(std::env::current_dir)
            .map_err(|error| PluginError::Failed(error.to_string()))?;
        if let Some(manager) = self.manager.get() {
            manager.start(&cwd, host).await;
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
