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
    valid_name(name)?;
    let index = super::dir::index(scope);
    let text = match std::fs::read_to_string(&index) {
        Ok(text) => text,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => String::new(),
        Err(error) => return Err(io(error)),
    };
    let next = super::index::unlink(&text, &format!("{name}.md"));
    std::fs::remove_file(scope.join(format!("{name}.md"))).map_err(io)?;
    replace(&index, &next).map_err(io)
}

fn replace(path: &Path, text: &str) -> std::io::Result<()> {
    static NEXT: AtomicU64 = AtomicU64::new(0);
    let temp = path.with_file_name(format!(
        ".MEMORY-{}-{}.tmp",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    ));
    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&temp)?;
    let result = (|| {
        file.write_all(text.as_bytes())?;
        file.sync_all()?;
        drop(file);
        std::fs::rename(&temp, path)
    })();
    if result.is_err() {
        let _ = std::fs::remove_file(&temp);
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn forgetting_replaces_an_existing_index_and_cleans_its_temp() {
        let home = tempfile::tempdir().unwrap();
        let scope = home.path().join("memory");
        std::fs::create_dir(&scope).unwrap();
        std::fs::write(scope.join("rule.md"), "fact").unwrap();
        std::fs::write(
            scope.join("MEMORY.md"),
            "# Memory\r\n[rule](rule.md)\r\n\r\nkeep\n",
        )
        .unwrap();
        forget(&scope, "rule").unwrap();
        assert_eq!(
            std::fs::read_to_string(scope.join("MEMORY.md")).unwrap(),
            "# Memory\r\n\r\nkeep\n"
        );
        assert_eq!(std::fs::read_dir(&scope).unwrap().count(), 1);
        assert_eq!(
            forget(&scope, "rule").unwrap_err().code,
            ErrorCode::NotFound
        );
        for name in ["../rule", "MEMORY", "a/b", "a\\b", "", "a:", "rule."] {
            assert_eq!(
                forget(&scope, name).unwrap_err().code,
                ErrorCode::InvalidInput
            );
        }
    }
}
