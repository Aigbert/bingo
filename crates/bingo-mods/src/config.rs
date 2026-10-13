//! Resource limits belong to the executor's own settings claim.
use crate::engine::Limits;
use schemars::JsonSchema;
use serde::Deserialize;
use std::{
    num::{NonZeroU64, NonZeroUsize},
    time::Duration,
};

#[derive(Default, Deserialize, JsonSchema)]
#[serde(default)]
pub(crate) struct Settings {
    pub mods: Options,
}
#[derive(Deserialize, JsonSchema)]
#[serde(default, rename_all = "camelCase")]
pub(crate) struct Options {
    #[serde(rename = "memoryMiB")]
    pub memory_mib: NonZeroUsize,
    pub hook_ms: NonZeroU64,
    pub every_ms: NonZeroU64,
}
impl Default for Options {
    fn default() -> Self {
        Self {
            memory_mib: NonZeroUsize::new(64).unwrap_or(NonZeroUsize::MIN),
            hook_ms: NonZeroU64::new(10_000).unwrap_or(NonZeroU64::MIN),
            every_ms: NonZeroU64::new(1000).unwrap_or(NonZeroU64::MIN),
        }
    }
}
impl Options {
    pub fn limits(self) -> Result<Limits, bingo_sdk::PluginError> {
        let memory_bytes = self
            .memory_mib
            .get()
            .checked_mul(1024 * 1024)
            .ok_or_else(|| bingo_sdk::PluginError::Config("mods.memoryMiB is too large".into()))?;
        Ok(Limits {
            memory_bytes,
            every_time: Duration::from_millis(self.every_ms.get()),
            hook_time: Duration::from_millis(self.hook_ms.get()),
            ..Limits::default()
        })
    }
}
