//! Explicit launch paths must name bytes covered by the directory digest.
use std::{io, path::Path};

pub(super) fn contained(root: &Path) -> io::Result<()> {
    let manifest: serde_json::Value =
        serde_json::from_slice(&std::fs::read(root.join("plugin.json"))?)?;
    let Some(entry) = manifest.get("entry") else {
        return Ok(());
    };
    let root = root.canonicalize()?;
    for raw in entry
        .get("command")
        .and_then(serde_json::Value::as_str)
        .into_iter()
        .chain(
            entry
                .get("args")
                .and_then(serde_json::Value::as_array)
                .into_iter()
                .flatten()
                .filter_map(serde_json::Value::as_str),
        )
    {
        check(&root, raw)?;
    }
    Ok(())
}

fn check(root: &Path, raw: &str) -> io::Result<()> {
    let expanded = raw.replace("${PLUGIN_ROOT}", &root.to_string_lossy());
    if !expanded.contains(['/', '\\']) && !raw.starts_with("${PLUGIN_ROOT}") {
        return Ok(());
    }
    // A foreign separator must not become a harmless filename on this host.
    #[cfg(not(windows))]
    if expanded.contains('\\') {
        return Err(io::Error::other(format!(
            "entry path {raw:?} uses a foreign separator"
        )));
    }
    let path = root.join(&expanded).canonicalize().map_err(|error| {
        io::Error::other(format!("entry path {raw:?} cannot be resolved: {error}"))
    })?;
    if !path.starts_with(root) {
        return Err(io::Error::other(format!(
            "entry path {raw:?} escapes the plugin root"
        )));
    }
    Ok(())
}
