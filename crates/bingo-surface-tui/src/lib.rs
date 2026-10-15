//! The full-screen terminal surface.
//!
//! It is a client of the one event stream like every other surface (ADR-0002):
//! it opens a session, folds the frames with `SessionState::apply`, and derives
//! every view at render time. It holds no session state of its own — what it
//! owns is the caret, the scroll, what is armed and what is open ([`ui::Ui`]) —
//! and it never awaits the kernel from the loop.
//!
//! # The shape of it
//!
//! - [`terminal`] takes the terminal and gives it back, on every path
//!   including a panic.
//! - [`run`] is the loop: `select!` over the keyboard, the frame stream, a
//!   tick, and the results of the handful of host calls it spawns.
//! - [`input::on_key`] and [`pointer::on_mouse`] are pure: they mutate
//!   [`ui::Ui`] and return [`effect::Effect`]s, so every binding is a test
//!   with no runtime in it.
//! - [`layout::layout`] cuts regions and composes their cells and targets;
//!   [`layout::paint`] copies them to the terminal; the transcript is what is left once the input box and the
//!   status line have taken theirs, so nothing below the transcript moves.
//! - [`tree::Tree`] holds one reducer state per session the attachment
//!   carries — the root and its sub-sessions (ADR-0010 §3) — and which of
//!   them is on screen.
//! - [`views`] is one renderer per node of the `View` vocabulary (ADR-0013);
//! - [`transcript`], [`markdown`], [`dialog`], [`composer`], [`commands`],
//!   [`keys`], [`paths`], [`welcome`] and [`wrap`] are the
//!   bricks those stand on, and [`theme`] is the one table of tokens and
//!   glyphs they draw with.
//! - [`status`] is the one line of furniture, and [`roster`] is the one list
//!   of sessions that `↓` and `ctrl+g` both open, with [`seats`] reading what
//!   a room says about the members in it.
//! - [`window`] is what every list a cursor walks draws when it outgrows its
//!   room: the rows around the cursor, and a `…` at each end it cut.
//! - [`complete`] is the `@` dropdown's ranking and [`mentions`] is who it can
//!   reach from the session on the screen — the agents under it, or, in a
//!   room, the seats on its roster.
//! - [`graphics`] asks the terminal whether it draws pictures and, where it
//!   does, turns one into the cells and the bytes that put it on the screen
//!   (design §5's image row).
//!
//! # What a person types
//!
//! `/help`, `/clear`, `/resume` and `/exit` are the surface's own (ADR-0008
//! §6). Every other `/name` and every `!line` is submitted verbatim: the
//! session actor parses commands, not the client.

mod actions;
/// The rows between the transcript and the input box: the verb row, the task
/// list and the queue (design §3).
mod activity;
mod ask;
mod blocks;
mod browse;
mod clipboard;
mod clock;
mod commands;
mod complete;
mod component;
mod composer;
mod count;
mod crew;
mod dialog;
mod effect;
mod face;
mod focus;
mod fold;
mod foot;
mod form;
mod frame;
mod graphics;
/// What this surface says about itself, as a page a model may read.
pub mod guide;
mod highlight;
mod history;
mod input;
mod keys;
/// An answer that came back after the probe that asked for it gave up (M60).
mod late;
mod layers;
mod layout;
mod lifecycle;
mod markdown;
mod matching;
mod mentions;
/// The opening (M149): the wordmark arriving, out of the motions the
/// product already has.
mod opening;
mod pager;
mod palette;
mod pane;
mod paths;
mod pictures;
mod pointer;
mod preview;
mod pulled;
mod render;
mod rewind;
mod run;
mod scroll;
mod search;
mod seats;
mod select;
mod sessions;
/// What a person may set about this surface: the measure prose is drawn to,
/// and the check the welcome box says the answer to.
pub mod settings;
/// The shells the session has running in the background (M75): a word under
/// the row that started one, a count on the status line, and no card.
mod status;
/// The session's task list, under the activity row and in its place between
/// turns (M74): where Claude Code keeps it.
mod terminal;
mod theme;
/// What a run of thoughts is (M79): consecutive reasoning items, which the
/// transcript draws as one thought.
mod thoughts;
mod toast;
mod transcript;
mod tree;
mod ui;
mod view;
/// Handing a picture to whatever this system opens pictures with (M56).
mod viewer;
mod views;
mod welcome;
mod window;
mod worked;
mod wrap;

use std::sync::Arc;

use async_trait::async_trait;
use bingo_sdk::{
    ConfigClaim, Exit, HostHandle, KernelError, Merge, Plugin, PluginError, PluginManifest,
    Registrar, Surface, SurfaceKind, SurfaceOptions,
};

/// How long the graphics probe waits for an answer that has tmux to cross, so
/// the pty harness's late scene can outlast the same window the surface uses.
pub use theme::PROBE_THROUGH;

/// The surface id, and the origin every input it submits carries.
pub const SURFACE_ID: &str = "tui";

#[derive(Debug, Default, Clone, Copy)]
pub struct TuiSurface {
    options: Options,
    settings: settings::Settings,
}

#[derive(Debug, Clone, Copy)]
pub struct Options {
    pub print_on_exit: bool,
    pub picture_cache_days: Option<u64>,
}

impl Default for Options {
    fn default() -> Self {
        Self {
            print_on_exit: true,
            picture_cache_days: None,
        }
    }
}

impl Options {
    pub(crate) fn cache_days(self) -> u64 {
        self.picture_cache_days
            .unwrap_or(bingo_pictures::cache::DAYS)
    }
}

#[async_trait]
impl Surface for TuiSurface {
    fn id(&self) -> &str {
        SURFACE_ID
    }

    fn kind(&self) -> SurfaceKind {
        SurfaceKind::Exclusive
    }

    async fn run(&self, host: HostHandle, opts: SurfaceOptions) -> Result<Exit, KernelError> {
        let print = self.options.print_on_exit;
        let mut screen = terminal::Tui::enter().map_err(entering)?;
        let ended = run::drive(&host, opts, *self, &mut screen, run::terminal_keys()).await;
        // The terminal goes back whether the loop ended or failed.
        let _ = screen.leave();
        let ended = ended?;
        if print {
            // The alternate screen took the conversation with it; this puts
            // the last screenful of it back where the shell can see it.
            for line in ended.screen {
                println!("{line}");
            }
            println!("{}", way_back(&ended.session));
        }
        Ok(ended.exit)
    }
}

/// The line under the last screenful: how to reopen the session just left.
fn way_back(session: &bingo_sdk::SessionId) -> String {
    format!("\nResume this session with:\n  bingo --resume {session}")
}

fn entering(e: std::io::Error) -> KernelError {
    KernelError::new(
        bingo_sdk::ErrorCode::Internal,
        format!("could not take the terminal: {e}"),
    )
}

pub(crate) static MANIFEST: PluginManifest = PluginManifest {
    id: "bingo.surface.tui",
    version: env!("CARGO_PKG_VERSION"),
    uses: &[],
    provides: &["surface:tui", "service:bingo.surface.tui.pages"],
    requires: &[],
    config: Some(ConfigClaim {
        approve: &[],
        keys: &[
            (bingo_update::SETTING, Merge::Replace),
            (settings::SETTING, Merge::Replace),
        ],
        schema: settings::schema,
    }),
};

#[derive(Debug, Default, Clone, Copy)]
pub struct TuiPlugin {
    options: Options,
}

impl TuiPlugin {
    pub fn new(options: Options) -> Self {
        Self { options }
    }

    fn surface(&self, registrar: &Registrar) -> Result<TuiSurface, PluginError> {
        Ok(TuiSurface {
            options: self.options,
            settings: registrar.config()?,
        })
    }
}

#[async_trait]
impl Plugin for TuiPlugin {
    fn manifest(&self) -> &'static PluginManifest {
        &MANIFEST
    }

    fn register(&self, registrar: &mut Registrar) -> Result<(), PluginError> {
        registrar.surface(Arc::new(self.surface(registrar)?) as Arc<dyn Surface>);
        registrar.add(guide::contribution(registrar));
        Ok(())
    }
}

#[cfg(test)]
mod plugin_tests {
    use super::*;
    use bingo_sdk::{Contribution, Env, Pages};

    #[test]
    fn registration_retains_both_claimed_settings() {
        let registrar = Registrar::new(
            MANIFEST.id,
            serde_json::json!({"tui": {"measure": 72}, "update": {"check": false}}),
            Env::rooted(std::env::temp_dir()),
            Default::default(),
        );
        let surface = TuiPlugin::default().surface(&registrar).unwrap();
        assert_eq!(surface.settings.measure(), Some(72));
        assert!(!surface.settings.update.check);
    }

    #[test]
    fn registration_validates_the_claimed_settings() {
        let mut registrar = Registrar::new(
            MANIFEST.id,
            serde_json::json!({"tui": {"measure": "wide"}}),
            Env::rooted(std::env::temp_dir()),
            Default::default(),
        );
        assert!(TuiPlugin::default().register(&mut registrar).is_err());
    }

    #[test]
    fn the_manifest_says_what_it_provides_and_the_two_keys_it_claims() {
        assert_eq!(MANIFEST.id, "bingo.surface.tui");
        let claim = MANIFEST.config.expect("a config claim");
        assert_eq!(claim.keys.len(), 2);
    }

    #[test]
    fn the_plugin_registers_the_surface_and_its_page() {
        let mut registrar = Registrar::new(
            MANIFEST.id,
            serde_json::json!({}),
            Env::rooted(std::env::temp_dir().join("bingo-tui-test")),
            Default::default(),
        );
        TuiPlugin::default()
            .register(&mut registrar)
            .expect("register");
        let contributions = registrar.into_contributions();
        assert_eq!(contributions.len(), MANIFEST.provides.len());
        match &contributions[0] {
            Contribution::Surface(surface) => assert_eq!(surface.id(), SURFACE_ID),
            other => panic!("expected the surface, got {other:?}"),
        }
        match &contributions[1] {
            Contribution::Service { key, wire, .. } => {
                assert_eq!(key, &Pages::key(MANIFEST.id).name());
                assert!(wire.is_none(), "a page is read in process");
            }
            other => panic!("expected the page service, got {other:?}"),
        }
    }
}

#[cfg(test)]
mod doubles;
#[cfg(test)]
mod motion;
#[cfg(test)]
mod painted;
#[cfg(test)]
mod screens;
#[cfg(test)]
mod test_lanes;
#[cfg(test)]
mod test_support;

mod embers;
#[cfg(test)]
mod identity;

#[cfg(test)]
mod crew_contract;

/// Draw a mod test's terminal tree without exposing terminal-library types.
pub fn drawn(view: &bingo_sdk::View, width: usize) -> Vec<String> {
    views::render(&test_engine_nodes(view), width)
        .into_iter()
        .map(|line| line.to_string().trim_end().to_owned())
        .collect()
}
fn test_engine_nodes(view: &bingo_sdk::View) -> bingo_sdk::View {
    use bingo_sdk::{View, view::TextChild};
    let mut view = view.clone();
    match &mut view {
        View::Engine { component, .. } => return View::text(format!("⟨{component:?}⟩")),
        View::Box { children, .. } => {
            for child in children {
                *child = test_engine_nodes(child);
            }
        }
        View::Panel { child, .. } => **child = test_engine_nodes(child),
        View::Text { children, .. } | View::Button { children, .. } => {
            for child in children {
                if let TextChild::Node(child) = child {
                    **child = test_engine_nodes(child);
                }
            }
        }
        _ => (),
    }
    view
}
#[cfg(test)]
mod author_kit_tests {
    #[test]
    fn drawn_uses_terminal_layout_and_marks_nested_engine_nodes() {
        let view = bingo_sdk::View::column(vec![
            bingo_sdk::View::text("header"),
            bingo_sdk::View::Engine {
                component: bingo_sdk::render::Component::AbovePrompt,
                props: serde_json::json!({}),
            },
        ]);
        assert_eq!(super::drawn(&view, 80), ["header", "⟨AbovePrompt⟩"]);
    }
}

#[cfg(test)]
mod person_surface {
    /// An unstamped line from here is the person's only if the sdk says so.
    #[test]
    fn this_surface_is_one_a_person_types_through() {
        assert!(bingo_sdk::PERSON_SURFACES.contains(&super::SURFACE_ID));
    }
}
