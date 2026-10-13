use bingo_sdk::{Env, KernelError};
use std::path::Path;

pub async fn run(root: &Path) -> Result<(), KernelError> {
    let home = tempfile::tempdir().map_err(super::invalid)?;
    for finding in bingo_plugin_rpc::validate::validate(root, &Env::rooted(home.path())).await? {
        println!("{finding}");
    }
    Ok(())
}
