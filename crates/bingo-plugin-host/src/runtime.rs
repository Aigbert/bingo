//! One launch form selects one executor.
use crate::manifest::Manifest;
use bingo_sdk::PluginRuntime;
pub fn runtime_of(_manifest: &Manifest, _mods_stand: bool) -> Result<PluginRuntime, String> {
    Ok(PluginRuntime::Process)
}
