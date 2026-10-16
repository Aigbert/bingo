//! Author types, written only when an author asks: a run never writes into a
//! plugin's directory. The catalogs come from a disposable host composed of
//! every plugin this build ships, in a home and project of its own, so the
//! person's settings, plugins and MCP servers are never read or spawned.
use crate::composition::Composition;
use bingo_core::{Host, HostConfig};
use bingo_plugin_host::manifest::Launch;
use bingo_sdk::{Catalog, CatalogKind, Env, HostApi, KernelError};
use std::path::{Path, PathBuf};

/// `bingo plugins types <dir>`: one written path per line, none when every
/// file already held its bytes.
pub async fn run(root: &Path) -> Result<(), KernelError> {
    let root = root.canonicalize().map_err(super::invalid)?;
    let manifest = bingo_plugin_rpc::validate::read(&root)?;
    if !matches!(manifest.launch, Launch::Module(_)) {
        return Err(super::invalid(format!(
            "{}: types are for module plugins; this one runs a process",
            manifest.name
        )));
    }
    for path in write(&root).await? {
        println!("{}", path.display());
    }
    Ok(())
}

/// The one place author types are laid: `types`, `test` and `validate` all
/// come here. Returns the files whose bytes changed.
pub async fn write(root: &Path) -> Result<Vec<PathBuf>, KernelError> {
    let (events, tools) = catalogs().await?;
    bingo_mods::types::write(root, &events, &tools).map_err(super::invalid)
}

async fn catalogs() -> Result<(Catalog, Catalog), KernelError> {
    let home = tempfile::tempdir().map_err(super::invalid)?;
    let cwd = home.path().join("project");
    std::fs::create_dir(&cwd).map_err(super::invalid)?;
    let env = Env::rooted(home.path());
    let plugins = crate::plugins(Composition::default(), &crate::platforms(&env), &cwd)?;
    let host = Host::build(plugins, HostConfig::new(env))
        .await
        .map_err(super::invalid)?;
    let events = host.catalog(CatalogKind::Events).await;
    let tools = host.catalog(CatalogKind::Tools).await;
    host.shutdown().await;
    Ok((events?, tools?))
}
