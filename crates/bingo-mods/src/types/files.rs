use std::path::{Component, Path};
pub(super) fn changed(root: &Path, relative: &str, bytes: &[u8]) -> Result<(), String> {
    let path = root.join(relative);
    let parent = Path::new(relative)
        .parent()
        .ok_or("type path has no parent")?;
    let mut current = root.to_path_buf();
    for part in parent.components() {
        let Component::Normal(part) = part else {
            return Err("invalid type directory".into());
        };
        current.push(part);
        match std::fs::symlink_metadata(&current) {
            Ok(meta) if meta.file_type().is_symlink() || !meta.is_dir() => {
                return Err(format!("{} is not a plain directory", current.display()));
            }
            Ok(_) => (),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                std::fs::create_dir(&current).map_err(|e| e.to_string())?
            }
            Err(error) => return Err(error.to_string()),
        }
    }
    if std::fs::symlink_metadata(&path).is_ok_and(|meta| meta.file_type().is_symlink()) {
        return Err(format!("{} is a symlink", path.display()));
    }
    if std::fs::read(&path).is_ok_and(|held| held == bytes) {
        return Ok(());
    }
    std::fs::write(&path, bytes).map_err(|e| format!("{}: {e}", path.display()))
}
