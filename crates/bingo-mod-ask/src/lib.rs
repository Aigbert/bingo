//! Built-in question and form interaction owner.
mod cleanup;
pub mod drawing;
mod guide;
mod handlers;
pub mod state;
use bingo_sdk::*;
use std::{
    collections::BTreeMap,
    sync::{Arc, Mutex},
};

type States = Arc<Mutex<BTreeMap<(SessionId, String), state::State>>>;
#[derive(Default)]
pub struct Ask;
static MANIFEST: PluginManifest = PluginManifest {
    id: "ask",
    version: env!("CARGO_PKG_VERSION"),
    uses: &[],
    requires: &[],
    provides: &["command:ask:ui", "service:ask.pages"],
    config: None,
};
impl Plugin for Ask {
    fn manifest(&self) -> &'static PluginManifest {
        &MANIFEST
    }
    fn register(&self, registrar: &mut Registrar) -> Result<(), PluginError> {
        let states = States::default();
        registrar.add(Contribution::Command(Arc::new(handlers::Handler(
            states.clone(),
        ))));
        registrar.add(Contribution::Hook(Arc::new(cleanup::Cleanup(
            states.clone(),
        ))));
        registrar.add(Contribution::Interceptor {
            placement: plugin::Placement::Builtin,
            external: None,
            link: Arc::new(drawing::Drawing(states)),
        });
        registrar.add(guide::contribution(registrar));
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn interaction_resolution_has_a_cleanup_observer() {
        let mut registrar = Registrar::new(
            "ask",
            serde_json::json!({}),
            Env::rooted("/unused"),
            Default::default(),
        );
        Ask.register(&mut registrar).unwrap();
        assert!(registrar.into_contributions().iter().any(
            |c| matches!(c, Contribution::Hook(h) if h.matcher().points.contains(&HookPoint::Event))
        ));
    }
}
