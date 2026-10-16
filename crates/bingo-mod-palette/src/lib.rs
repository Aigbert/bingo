//! The shared palette's commands, navigation and drawing.
pub mod args;
pub mod draw;
mod guide;
mod hints;
mod keys;
mod lifecycle;
mod line;
pub mod rows;
mod skills;
mod uses;
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
    /// The folds opened at this query; a new query closes them again.
    expanded: std::collections::BTreeSet<String>,
}
struct State {
    cursors: BTreeMap<(SessionId, String), Cursor>,
    uses: uses::Uses,
}
impl State {
    /// The cursor of one drawing, the groups it walks, and the row it rests
    /// on: a drawing begun or a query changed starts at the initial row with
    /// every fold closed.
    fn walk(
        &mut self,
        session: &SessionId,
        request: &str,
        props: &component::PaletteProps,
    ) -> (&mut Cursor, Vec<rows::Group>) {
        let top = self.uses.top();
        let key = (session.clone(), request.to_owned());
        let begun = !self.cursors.contains_key(&key);
        let cursor = self.cursors.entry(key).or_default();
        let fresh = begun || cursor.query != props.query;
        if fresh {
            cursor.query = props.query.clone();
            cursor.expanded.clear();
        }
        let (groups, initial) = draw::groups(props, &top, &cursor.expanded);
        if fresh {
            cursor.selected = initial;
        }
        let rows = groups
            .iter()
            .flat_map(|group| &group.rows)
            .collect::<Vec<_>>();
        cursor.selected = cursor.selected.min(rows.len().saturating_sub(1));
        if rows
            .get(cursor.selected)
            .is_some_and(|row| matches!(row.target, rows::Target::Inert))
        {
            cursor.selected = rows
                .iter()
                .position(|row| !matches!(row.target, rows::Target::Inert))
                .unwrap_or(0);
        }
        (cursor, groups)
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
            uses: uses::Uses::load(uses::path(&registrar.env().data_dir)),
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
