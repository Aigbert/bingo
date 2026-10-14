//! A complete built-in feature, used only by tests.
mod command;
mod drawing;
mod handlers;
use bingo_sdk::*;
use std::sync::Arc;

pub struct Hello;
static MANIFEST: PluginManifest = PluginManifest {
    id: "hello",
    version: env!("CARGO_PKG_VERSION"),
    uses: &[],
    requires: &[],
    provides: &["command:hello", "command:hello:ui"],
    config: None,
};
impl Plugin for Hello {
    fn manifest(&self) -> &'static PluginManifest {
        &MANIFEST
    }
    fn register(&self, registrar: &mut Registrar) -> Result<(), PluginError> {
        registrar.add(Contribution::Command(Arc::new(command::Hello)));
        registrar.add(Contribution::Command(Arc::new(handlers::Handler)));
        registrar.add(Contribution::Interceptor {
            placement: plugin::Placement::Builtin,
            external: None,
            link: Arc::new(drawing::Drawing),
        });
        Ok(())
    }
}
