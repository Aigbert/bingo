//! The shared palette's commands, navigation and drawing.
mod args;
mod draw;
mod guide;
mod keys;
mod lifecycle;
mod line;
mod resume;
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
}
impl State {
    /// The cursor of one drawing, what it lists, and the row it rests on: a
    /// drawing begun or a query changed starts at the listing's first row.
    fn walk(
        &mut self,
        session: &SessionId,
        request: &str,
        props: &component::PaletteProps,
    ) -> (&mut Cursor, draw::Listing) {
        let key = (session.clone(), request.to_owned());
        let begun = !self.cursors.contains_key(&key);
        let cursor = self.cursors.entry(key).or_default();
        let listing = draw::listing(props);
        if begun || cursor.query != props.query {
            cursor.query = props.query.clone();
            cursor.selected = listing.initial;
        }
        cursor.selected = cursor.selected.min(listing.rows.len().saturating_sub(1));
        (cursor, listing)
    }
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
