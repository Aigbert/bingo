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
    fn is_current(&self) -> bool {
        true
    }
    fn generation(&self) -> Option<bingo_sdk::ItemId> {
        None
    }
    async fn prepare(&self) -> Result<(), String>;
    async fn call(&self, id: &str, input: Value, cx: &HookContext) -> Result<Value, String>;
}
#[derive(Default)]
pub struct Registry(RwLock<BTreeMap<String, Vec<Arc<dyn Executor>>>>);
impl Registry {
    pub fn insert(&self, plugin: String, executor: Arc<dyn Executor>) {
        let mut owners = self.0.write().unwrap_or_else(|error| error.into_inner());
        let entries = owners.entry(plugin).or_default();
        entries.retain(|entry| entry.generation() != executor.generation());
        entries.push(executor);
    }
    pub fn remove_generation(&self, plugin: &str, generation: &bingo_sdk::ItemId) {
        let mut owners = self.0.write().unwrap_or_else(|error| error.into_inner());
        if let Some(entries) = owners.get_mut(plugin) {
            entries.retain(|entry| entry.generation().as_ref() != Some(generation));
            if entries.is_empty() {
                owners.remove(plugin);
            }
        }
    }
    pub fn remove(&self, plugin: &str) {
        self.0
            .write()
            .unwrap_or_else(|e| e.into_inner())
            .remove(plugin);
    }
    pub fn get(&self, plugin: &str) -> Result<Arc<dyn Executor>, String> {
        self.0
            .read()
            .unwrap_or_else(|error| error.into_inner())
            .get(plugin)
            .and_then(|entries| {
                entries
                    .iter()
                    .rev()
                    .find(|executor| {
                        bingo_sdk::generation::current(plugin).map_or_else(
                            || executor.is_current(),
                            |id| executor.generation() == Some(id),
                        ) && !executor.stopped().is_cancelled()
                    })
                    .cloned()
            })
            .ok_or_else(|| format!("{plugin}: callback runtime is unavailable"))
    }
    pub async fn prepared(&self, plugin: &str) -> Result<Arc<dyn Executor>, String> {
        self.get(plugin)?.prepare().await?;
        self.get(plugin)
    }
}
