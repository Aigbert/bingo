//! The session directory owns its rows, verbs and drawing.
pub mod draw;
mod guide;
mod keys;
mod probe;
pub mod rows;
mod state;
pub mod title;
mod verbs;
mod welcome;
use bingo_sdk::*;
use std::sync::{Arc, Mutex};
#[derive(Default)]
pub struct Sessions;
type Shared = Arc<Mutex<state::State>>;
static MANIFEST: PluginManifest = PluginManifest {
    id: "bingo.mod.sessions",
    version: env!("CARGO_PKG_VERSION"),
    uses: &[],
    requires: &[],
    provides: &[
        "command:sessions",
        "command:sessions:ui",
        "service:bingo.mod.sessions.pages",
    ],
    config: None,
};
impl Plugin for Sessions {
    fn manifest(&self) -> &'static PluginManifest {
        &MANIFEST
    }
    fn register(&self, registrar: &mut Registrar) -> Result<(), PluginError> {
        let shared = Shared::default();
        registrar.add(Contribution::Command(Arc::new(keys::Open)));
        registrar.add(Contribution::Command(Arc::new(keys::Handler(
            shared.clone(),
        ))));
        for link in [
            Arc::new(draw::Drawing(shared.clone())) as Arc<dyn Interceptor>,
            Arc::new(title::Title),
            Arc::new(welcome::Welcome),
        ] {
            registrar.add(Contribution::Interceptor {
                placement: plugin::Placement::Builtin,
                external: None,
                link,
            });
        }
        registrar.add(Contribution::Hook(Arc::new(state::Cleanup(shared))));
        registrar.add(guide::contribution(registrar));
        Ok(())
    }
}
fn error(error: impl std::fmt::Display) -> KernelError {
    KernelError::new(ErrorCode::Internal, error.to_string())
}
#[cfg(test)]
mod tests;
