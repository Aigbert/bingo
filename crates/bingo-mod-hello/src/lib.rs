//! A complete built-in feature, used only by tests.
mod board;
mod command;
mod progress;
mod writer;
pub use board::board;
mod drawing;
mod handlers;
use bingo_sdk::*;
use std::sync::Arc;

pub const ENABLE_ENV: &str = "BINGO_FAKE_HELLO";

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

pub struct Harness;
static HARNESS: PluginManifest = PluginManifest {
    provides: &[
        "command:hello",
        "command:hello:ui",
        "command:hello-board",
        "command:hello.tick",
        "command:hello.reset",
        "hook:hello:board-row",
        "tool:HelloProgress",
    ],
    ..MANIFEST
};
impl Plugin for Harness {
    fn manifest(&self) -> &'static PluginManifest {
        &HARNESS
    }
    fn register(&self, registrar: &mut Registrar) -> Result<(), PluginError> {
        Hello.register(registrar)?;
        for name in ["hello-board", "hello.tick", "hello.reset"] {
            registrar.add(Contribution::Command(Arc::new(board::BoardCommand(name))));
        }
        registrar.add(Contribution::Hook(Arc::new(board::BoardRow)));
        registrar.tool(Arc::new(progress::Progress));
        Ok(())
    }
}

#[cfg(doctest)]
#[doc = include_str!("../../../docs/design/tui.md")]
struct DesignDoc;
