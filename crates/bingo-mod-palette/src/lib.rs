//! The shared palette's commands, navigation and drawing.
pub mod args;
pub mod draw;
mod guide;
mod hints;
mod keys;
mod lifecycle;
mod recent;
pub mod rows;
use bingo_sdk::*;
use std::{
    collections::BTreeMap,
    sync::{Arc, Mutex},
};

#[derive(Default)]
pub struct Palette;
#[derive(Default)]
struct Cursor {
    query: String,
    selected: usize,
    /// What the site was last drawn from, so a press is judged against the
    /// drawing the person saw without carrying it back.
    drawn: Option<Arc<component::PaletteProps>>,
}
struct State {
    cursors: BTreeMap<(SessionId, String), Cursor>,
    recent: recent::Recent,
}
type Shared = Arc<Mutex<State>>;
static MANIFEST: PluginManifest = PluginManifest {
    id: "bingo.mod.palette",
    version: env!("CARGO_PKG_VERSION"),
    uses: &[],
    requires: &[],
    provides: &[
        "command:help",
        "command:resume",
        "command:palette:ui",
        "service:bingo.mod.palette.pages",
    ],
    config: None,
};
impl Plugin for Palette {
    fn manifest(&self) -> &'static PluginManifest {
        &MANIFEST
    }
    fn register(&self, registrar: &mut Registrar) -> Result<(), PluginError> {
        let shared = Arc::new(Mutex::new(State {
            cursors: BTreeMap::new(),
            recent: recent::Recent::load(
                registrar.env().data_dir.join("plugins/palette/recent.json"),
            ),
        }));
        registrar.add(Contribution::Hook(Arc::new(lifecycle::Lifecycle {
            path: registrar.env().data_dir.join("plugins/palette/key-hint"),
            state: shared.clone(),
        })));
        for name in ["help", "resume"] {
            registrar.add(Contribution::Command(Arc::new(keys::Open(name))));
        }
        registrar.add(Contribution::Command(Arc::new(keys::Handler(
            shared.clone(),
        ))));
        registrar.add(Contribution::Interceptor {
            placement: plugin::Placement::Builtin,
            external: None,
            link: Arc::new(draw::Drawing(shared)),
        });
        registrar.add(guide::contribution(registrar));
        Ok(())
    }
}
fn error(error: impl std::fmt::Display) -> KernelError {
    KernelError::new(ErrorCode::Internal, error.to_string())
}
