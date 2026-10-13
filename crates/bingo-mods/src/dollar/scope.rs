//! Canonical paths keep each operation inside its declared roots.
use std::path::{Component, Path, PathBuf};

pub const LIMIT: usize = 4 * 1024 * 1024;
pub struct Roots<'a> {
    pub project: &'a Path,
    pub data: &'a Path,
}
#[derive(Clone, Copy)]
pub enum Access {
    Read,
    Write,
    GrantedWrite,
}
#[derive(Debug, thiserror::Error)]
#[error("{0}")]
pub struct Refusal(pub String);

pub fn cap(len: usize, limit: usize) -> Result<(), Refusal> {
    if len > limit {
        return Err(Refusal(format!(
            "{len} bytes exceeds the {limit} byte limit"
        )));
    }
    Ok(())
}

pub fn scope(path: &Path, roots: Roots<'_>, access: Access) -> Result<PathBuf, Refusal> {
    let project = canonical(roots.project)?;
    let data = canonical(roots.data)?;
    let path = canonical(&project.join(path))?;
    let inside_data = path.starts_with(&data);
    if !inside_data && !path.starts_with(&project) {
        return Err(Refusal(
            "path leaves the project and plugin data roots".into(),
        ));
    }
    if matches!(access, Access::Write) && !inside_data {
        return Err(Refusal(
            "writing outside the plugin data directory needs the fs grant".into(),
        ));
    }
    Ok(path)
}

fn lexical(path: &Path) -> Result<PathBuf, Refusal> {
    let mut result = PathBuf::new();
    for component in path.components() {
        match component {
            Component::CurDir => (),
            Component::ParentDir => {
                if !result.pop() {
                    return Err(Refusal("path leaves its root".into()));
                }
            }
            other => result.push(other.as_os_str()),
        }
    }
    Ok(result)
}

fn canonical(path: &Path) -> Result<PathBuf, Refusal> {
    let path = lexical(path)?;
    let mut ancestor = path.as_path();
    let mut missing = Vec::new();
    loop {
        match dunce::canonicalize(ancestor) {
            Ok(mut resolved) => {
                for name in missing.into_iter().rev() {
                    resolved.push(name);
                }
                return Ok(resolved);
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                // A dangling link must not masquerade as a new ordinary file.
                if std::fs::symlink_metadata(ancestor).is_ok() {
                    return Err(Refusal("path contains a dangling symbolic link".into()));
                }
                missing.push(
                    ancestor
                        .file_name()
                        .ok_or_else(|| Refusal(error.to_string()))?,
                );
                ancestor = ancestor
                    .parent()
                    .ok_or_else(|| Refusal(error.to_string()))?;
            }
            Err(error) => return Err(Refusal(error.to_string())),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn roots_bound_reads_and_writes_including_new_paths() {
        let root = tempfile::tempdir().unwrap();
        let project = root.path().join("project");
        let data = root.path().join("data");
        std::fs::create_dir(&project).unwrap();
        std::fs::create_dir(&data).unwrap();
        for (path, access, allowed) in [
            (project.join("new/file"), Access::Read, true),
            (data.join("new/file"), Access::Read, true),
            (data.join("new/file"), Access::Write, true),
            (project.join("file"), Access::Write, false),
            (project.join("file"), Access::GrantedWrite, true),
            (project.join("../outside"), Access::Read, false),
        ] {
            assert_eq!(
                scope(
                    &path,
                    Roots {
                        project: &project,
                        data: &data
                    },
                    access
                )
                .is_ok(),
                allowed,
                "{path:?}"
            );
        }
        assert_eq!(
            cap(LIMIT + 1, LIMIT).unwrap_err().to_string(),
            "4194305 bytes exceeds the 4194304 byte limit"
        );
    }
    #[cfg(unix)]
    #[test]
    fn symlinks_cannot_leave_either_root() {
        let root = tempfile::tempdir().unwrap();
        let project = root.path().join("project");
        std::fs::create_dir(&project).unwrap();
        std::os::unix::fs::symlink(root.path(), project.join("link")).unwrap();
        assert!(
            scope(
                &project.join("link/new"),
                Roots {
                    project: &project,
                    data: &project
                },
                Access::Read
            )
            .is_err()
        );
    }
    #[cfg(windows)]
    #[test]
    fn drive_and_verbatim_roots_compare_canonically() {
        let root = tempfile::tempdir().unwrap();
        let verbatim = root.path().canonicalize().unwrap();
        let plain = dunce::canonicalize(root.path()).unwrap();
        assert_eq!(
            scope(
                &verbatim,
                Roots {
                    project: &plain,
                    data: &plain
                },
                Access::Read
            )
            .unwrap(),
            plain
        );
        let unc = Path::new(r"\\server\share\folder\..\file");
        assert_eq!(lexical(unc).unwrap(), PathBuf::from(r"\\server\share\file"));
    }
}
