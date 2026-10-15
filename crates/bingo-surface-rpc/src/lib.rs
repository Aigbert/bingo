//! The JSON-RPC surface: the sdk with an envelope, and nothing else (ADR-0007).
//!
//! [`serve`] is `bingo serve --stdio`; [`RemoteKernel`] is the same contract
//! seen from the other end of the pipe; [`schema::document`] is both of them
//! written down as JSON Schema, committed at `schema/rpc.json`.

pub mod client;
pub mod codec;
pub mod methods;
pub mod schema;
pub mod server;
mod session;

use std::sync::Arc;

use async_trait::async_trait;
use bingo_sdk::{
    Exit, HostHandle, KernelError, Plugin, PluginError, PluginManifest, Registrar, Surface,
    SurfaceKind, SurfaceOptions,
};

pub use client::{RemoteKernel, RemoteSession};
pub use schema::document;
pub use server::serve;

/// The surface id.
pub const SURFACE_ID: &str = "rpc";

#[derive(Debug, Default, Clone, Copy)]
pub enum Transport {
    #[default]
    Stdio,
}

#[derive(Debug, Default, Clone, Copy)]
pub struct Options {
    pub transport: Transport,
}

/// Owns stdio, has no prompt and no session of its own.
#[derive(Debug, Default, Clone, Copy)]
pub struct RpcSurface {
    options: Options,
}

#[async_trait]
impl Surface for RpcSurface {
    fn id(&self) -> &str {
        SURFACE_ID
    }

    fn kind(&self) -> SurfaceKind {
        SurfaceKind::Exclusive
    }

    async fn run(&self, host: HostHandle, _opts: SurfaceOptions) -> Result<Exit, KernelError> {
        match self.options.transport {
            Transport::Stdio => serve(host, tokio::io::stdin(), tokio::io::stdout()).await,
        }
    }
}

static MANIFEST: PluginManifest = PluginManifest {
    id: "bingo.surface.rpc",
    version: env!("CARGO_PKG_VERSION"),
    uses: &[],
    provides: &["surface:rpc"],
    requires: &[],
    config: None,
};

#[derive(Debug, Default, Clone, Copy)]
pub struct RpcPlugin {
    options: Options,
}

impl RpcPlugin {
    pub fn new(options: Options) -> Self {
        Self { options }
    }
}

#[async_trait]
impl Plugin for RpcPlugin {
    fn manifest(&self) -> &'static PluginManifest {
        &MANIFEST
    }

    fn register(&self, registrar: &mut Registrar) -> Result<(), PluginError> {
        registrar.surface(Arc::new(RpcSurface {
            options: self.options,
        }) as Arc<dyn Surface>);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn the_surface_owns_stdio_alone() {
        assert_eq!(RpcSurface::default().id(), SURFACE_ID);
        assert_eq!(RpcSurface::default().kind(), SurfaceKind::Exclusive);
        assert_eq!(MANIFEST.provides, &["surface:rpc"]);
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
