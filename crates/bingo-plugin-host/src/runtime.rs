//! One launch form selects one executor.
use crate::manifest::{Launch, Manifest};
use bingo_sdk::PluginRuntime;
pub fn runtime_of(manifest: &Manifest, mods_stand: bool) -> Result<PluginRuntime, String> {
    match manifest.launch {
        Launch::Entry(_) => Ok(PluginRuntime::Process),
        Launch::Module(_) if mods_stand => Ok(PluginRuntime::Module),
        Launch::Module(_) => Err(format!("{}: module runtime is switched off", manifest.name)),
    }
}
