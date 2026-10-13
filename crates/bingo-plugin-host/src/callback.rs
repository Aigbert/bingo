//! Runtime callbacks have one owner and end with that owner's generation.
use bingo_sdk::{CancellationToken, HookContext, ServiceKey};
use serde_json::Value;
use std::{
    collections::BTreeMap,
    sync::{Arc, RwLock},
};

pub const KEY: ServiceKey<Registry> = ServiceKey::new("bingo.plugin-host.callbacks");
#[async_trait::async_trait]
pub trait Executor: Send + Sync {
    fn stopped(&self) -> CancellationToken;
    async fn prepare(&self) -> Result<(), String>;
    async fn call(&self, id: &str, input: Value, cx: &HookContext) -> Result<Value, String>;
}
#[derive(Default)]
pub struct Registry(RwLock<BTreeMap<String, Arc<dyn Executor>>>);
impl Registry {
    pub fn insert(&self, plugin: String, executor: Arc<dyn Executor>) {
        self.0
            .write()
            .unwrap_or_else(|error| error.into_inner())
            .insert(plugin, executor);
    }
    pub fn get(&self, plugin: &str) -> Result<Arc<dyn Executor>, String> {
        self.0
            .read()
            .unwrap_or_else(|error| error.into_inner())
            .get(plugin)
            .cloned()
            .filter(|executor| !executor.stopped().is_cancelled())
            .ok_or_else(|| format!("{plugin}: callback runtime is unavailable"))
    }
    pub async fn prepared(&self, plugin: &str) -> Result<Arc<dyn Executor>, String> {
        self.get(plugin)?.prepare().await?;
        self.get(plugin)
    }
}
