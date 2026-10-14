use bingo_sdk::{Env, KernelError};
use std::path::Path;

pub async fn run(root: &Path) -> Result<(), KernelError> {
    let root = root.canonicalize().map_err(super::invalid)?;
    let manifest = bingo_plugin_rpc::validate::read(&root)?;
    if matches!(
        manifest.launch,
        bingo_plugin_host::manifest::Launch::Module(_)
    ) {
        return module(&root, &manifest);
    }
    let home = tempfile::tempdir().map_err(super::invalid)?;
    for finding in bingo_plugin_rpc::validate::validate(&root, &Env::rooted(home.path())).await? {
        println!("{finding}");
    }
    Ok(())
}
fn module(
    root: &Path,
    manifest: &bingo_plugin_host::manifest::Manifest,
) -> Result<(), KernelError> {
    let output = bingo_mods::validate::validate(root, manifest).map_err(super::invalid)?;
    for finding in &output.findings {
        let rule = serde_json::to_value(&finding.rule).map_err(super::invalid)?;
        println!(
            "{}:{}:{} {} {}",
            finding.file,
            finding.line,
            finding.column,
            rule.as_str().unwrap_or_default(),
            finding.text
        );
    }
    for (label, values) in [
        ("event", &output.events),
        ("call", &output.calls),
        ("key", &output.keys),
        ("uses", &output.uses),
    ] {
        for value in values {
            println!("{label}: {value}");
        }
    }
    if output.valid() {
        Ok(())
    } else {
        Err(super::invalid("module validation failed"))
    }
}
