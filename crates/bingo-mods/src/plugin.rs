//! The module executor contributes late plugins and interception links.
use bingo_sdk::*;
use async_trait::async_trait;
use std::path::PathBuf;

#[derive(Clone, Default)]
pub struct ModsPlugin { directories: Vec<PathBuf> }
impl ModsPlugin {
    pub fn with_dirs(mut self, directories: Vec<PathBuf>) -> Self { self.directories = directories; self }
}
static MANIFEST: PluginManifest = PluginManifest {
    id: "bingo.mods", version: env!("CARGO_PKG_VERSION"), uses: &[], provides: &[], requires: &[], config: None,
};
#[async_trait]
impl Plugin for ModsPlugin {
    fn manifest(&self) -> &'static PluginManifest { &MANIFEST }
    fn register(&self, _: &mut Registrar) -> Result<(), PluginError> { Ok(()) }
}
