//! Forgetting is the only write offered to a person.
use bingo_sdk::{ErrorCode, KernelError};
use std::{
    io::Write,
    path::Path,
    sync::atomic::{AtomicU64, Ordering},
};

pub fn valid_name(name: &str) -> Result<(), KernelError> {
    if name.is_empty()
        || name.eq_ignore_ascii_case("MEMORY")
        || name.chars().any(|c| {
            c.is_control() || matches!(c, '/' | '\\' | ':' | '"' | '<' | '>' | '|' | '?' | '*')
        })
        || name.trim_end_matches([' ', '.']) != name
        || matches!(name, "." | "..")
    {
        return Err(KernelError::new(
            ErrorCode::InvalidInput,
            "expected a memory file name, not a path or index",
        ));
    }
    Ok(())
}

pub fn io(error: std::io::Error) -> KernelError {
    KernelError::new(
        if error.kind() == std::io::ErrorKind::NotFound {
            ErrorCode::NotFound
        } else {
            ErrorCode::Internal
        },
        error.to_string(),
    )
}

pub fn forget(scope: &Path, name: &str) -> Result<(), KernelError> {
    forget_with(scope, name, temporary, |_| {}, sync_directory)
}

fn temporary(scope: &Path) -> std::path::PathBuf {
    static NEXT: AtomicU64 = AtomicU64::new(0);
    scope.join(format!(
        ".MEMORY-{}-{}.tmp",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    ))
}

fn forget_with(
    scope: &Path,
    name: &str,
    mut temporary: impl FnMut(&Path) -> std::path::PathBuf,
    mut before_swap: impl FnMut(usize),
    sync: impl Fn(&Path) -> std::io::Result<()>,
) -> Result<(), KernelError> {
    valid_name(name)?;
    let file = scope.join(format!("{name}.md"));
    std::fs::symlink_metadata(&file).map_err(io)?;
    let index = super::dir::index(scope);
    for attempt in 0..2 {
        let original = read_index(&index).map_err(io)?;
        let next = super::index::unlink(
            original.as_deref().unwrap_or_default(),
            &format!("{name}.md"),
        );
        let temp = temporary(scope);
        if replace(&index, original.as_deref(), &next, &temp, || {
            before_swap(attempt)
        })
        .map_err(io)?
        {
            sync_after_rename(scope, sync);
            return std::fs::remove_file(file).map_err(io);
        }
    }
    Err(KernelError::new(
        ErrorCode::InvalidInput,
        "memory index changed twice; nothing forgotten; try again",
    ))
}

fn read_index(path: &Path) -> std::io::Result<Option<String>> {
    match std::fs::read_to_string(path) {
        Ok(text) => Ok(Some(text)),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(error),
    }
}

fn replace(
    path: &Path,
    original: Option<&str>,
    text: &str,
    temp: &Path,
    before_swap: impl FnOnce(),
) -> std::io::Result<bool> {
    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(temp)?;
    let result = (|| {
        file.write_all(text.as_bytes())?;
        file.sync_all()?;
        drop(file);
        before_swap();
        if read_index(path)?.as_deref() != original {
            return Ok(false);
        }
        std::fs::rename(temp, path)?;
        Ok(true)
    })();
    if !matches!(result, Ok(true)) {
        let _ = std::fs::remove_file(temp);
    }
    result
}

/// The rename has already happened; a failed sync only weakens durability, so the
/// forget goes on to remove the file the index no longer points at.
fn sync_after_rename(scope: &Path, sync: impl Fn(&Path) -> std::io::Result<()>) {
    if let Err(error) = sync(scope) {
        tracing::warn!(%error, "memory: the index directory was not synced");
    }
}

#[cfg(unix)]
fn sync_directory(path: &Path) -> std::io::Result<()> {
    std::fs::File::open(path)?.sync_all()
}
#[cfg(not(unix))]
fn sync_directory(_: &Path) -> std::io::Result<()> {
    // Windows has no portable directory fsync; the replacement file is already synced.
    Ok(())
}

#[cfg(test)]
mod tests;
