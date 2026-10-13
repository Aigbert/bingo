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

pub fn resolve_with(_env: &dyn ShellEnv, _platform: Platform) -> Result<Shell, ShellMissing> {
    Err(ShellMissing { tried: Vec::new() })
}

#[cfg(test)]
mod tests;
