//! The POSIX shell selected once for tools and hooks (ADR-0063).
use std::ffi::OsString;
use std::path::{Path, PathBuf};

/// Environment inputs, injectable without changing process-global variables.
pub trait ShellEnv {
    fn var(&self, name: &str) -> Option<OsString>;
    fn path(&self) -> Vec<PathBuf>;
    fn is_file(&self, path: &Path) -> bool;
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Platform {
    Unix,
    Windows,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Shell {
    path: PathBuf,
}

impl Shell {
    pub fn path(&self) -> &Path {
        &self.path
    }
}

/// Every attempted location and the same remedy for both consumers.
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
#[error(
    "No POSIX shell found (tried: {tried:?}). Install Git for Windows or set BINGO_SHELL to an absolute POSIX shell path."
)]
pub struct ShellMissing {
    pub tried: Vec<PathBuf>,
}

/// Resolve without mutating the environment; both platforms are testable anywhere.
pub fn resolve_with(env: &dyn ShellEnv, platform: Platform) -> Result<Shell, ShellMissing> {
    if let Some(path) = env.var("BINGO_SHELL") {
        let path = PathBuf::from(path);
        return if absolute(&path, platform) && env.is_file(&path) {
            Ok(Shell { path })
        } else {
            Err(ShellMissing { tried: vec![path] })
        };
    }
    let candidates = match platform {
        Platform::Unix => vec![PathBuf::from("/bin/bash"), PathBuf::from("/bin/sh")],
        Platform::Windows => windows_candidates(env),
    };
    let mut tried = Vec::new();
    for path in candidates {
        if env.is_file(&path) {
            return Ok(Shell { path });
        }
        tried.push(path);
    }
    Err(ShellMissing { tried })
}

fn absolute(path: &Path, platform: Platform) -> bool {
    match platform {
        Platform::Unix => path.as_os_str().as_encoded_bytes().starts_with(b"/"),
        Platform::Windows => {
            let path = windows_path(path);
            let bytes = path.as_os_str().as_encoded_bytes();
            bytes.starts_with(b"//")
                || (bytes.len() >= 3 && bytes[0].is_ascii_alphabetic() && &bytes[1..3] == b":/")
        }
    }
}

fn windows_candidates(env: &dyn ShellEnv) -> Vec<PathBuf> {
    let paths: Vec<_> = env.path().iter().map(|path| windows_path(path)).collect();
    let root = env.var("SystemRoot").map(PathBuf::from);
    let mut candidates: Vec<_> = paths
        .iter()
        .filter(|path| !wsl(path, root.as_deref()))
        .map(|path| path.join("bash.exe"))
        .collect();
    for (key, suffix) in [
        ("ProgramFiles", "Git/bin/bash.exe"),
        ("ProgramFiles(x86)", "Git/bin/bash.exe"),
        ("LocalAppData", "Programs/Git/bin/bash.exe"),
    ] {
        if let Some(dir) = env.var(key).filter(|value| !value.is_empty()) {
            candidates.push(windows_path(Path::new(&dir)).join(suffix));
        }
    }
    for path in paths {
        if env.is_file(&path.join("git.exe"))
            && let Some(parent) = path.parent()
        {
            candidates.push(parent.join("bin/bash.exe"));
        }
    }
    candidates
}

// Windows accepts either separator. Normalizing also lets another host test
// Windows paths without applying that host's lexical path rules to backslashes.
fn windows_path(path: &Path) -> PathBuf {
    match path.to_str() {
        Some(path) => PathBuf::from(path.replace('\\', "/")),
        None => path.to_path_buf(),
    }
}

fn wsl(path: &Path, root: Option<&Path>) -> bool {
    let path = path.to_string_lossy().to_ascii_lowercase();
    if path.split('/').any(|part| part == "windowsapps") {
        return true;
    }
    root.is_some_and(|root| {
        let root = windows_path(root).to_string_lossy().to_ascii_lowercase();
        let root = root.trim_end_matches('/');
        !root.is_empty() && (path == root || path.starts_with(&format!("{root}/")))
    })
}

struct Environment;

impl ShellEnv for Environment {
    fn var(&self, name: &str) -> Option<OsString> {
        std::env::var_os(name)
    }
    fn path(&self) -> Vec<PathBuf> {
        std::env::var_os("PATH")
            .map(|path| std::env::split_paths(&path).collect())
            .unwrap_or_default()
    }
    fn is_file(&self, path: &Path) -> bool {
        path.is_file()
    }
}

/// One immutable answer for every consumer throughout the process lifetime.
pub fn resolve() -> Result<Shell, ShellMissing> {
    static SHELL: std::sync::OnceLock<Result<Shell, ShellMissing>> = std::sync::OnceLock::new();
    SHELL
        .get_or_init(|| {
            let platform = if cfg!(windows) {
                Platform::Windows
            } else {
                Platform::Unix
            };
            resolve_with(&Environment, platform)
        })
        .clone()
}

#[cfg(test)]
mod tests;
