//! Persistent ownership of a name, released by the OS when its holder exits.
use std::fs::File;
use std::io;
use std::path::{Path, PathBuf};

/// Immutable bytes identifying a permanent claim file.
#[derive(Clone, Copy, Debug)]
pub struct Protocol<'a> {
    pub file: &'a str,
    pub marker: &'a str,
}

#[derive(Debug, thiserror::Error)]
pub enum ClaimError {
    #[error("another runner holds {}", path.display())]
    WouldBlock { path: PathBuf },
    #[error(
        "legacy or unrecognized runner lock ({}); stop all bingo processes sharing this store, run one `bingo gateway doctor --fix` for a dead PID sentinel, then restart; otherwise inspect the file before removing it",
        path.display()
    )]
    Legacy { path: PathBuf },
    #[error("{}: {source}", path.display())]
    Io {
        path: PathBuf,
        #[source]
        source: io::Error,
    },
}

#[derive(Debug)]
pub struct Claim {
    path: PathBuf,
    _file: File,
}

impl Claim {
    pub fn take(dir: &Path, protocol: Protocol<'_>) -> Result<Self, ClaimError> {
        Err(ClaimError::Io {
            path: dir.join(protocol.file),
            source: io::Error::other("claim protocol not implemented"),
        })
    }

    pub fn path(&self) -> &Path {
        &self.path
    }
}

pub fn probe(dir: &Path, protocol: Protocol<'_>) -> Result<bool, ClaimError> {
    Claim::take(dir, protocol).map(|_| false)
}

pub fn holder(_dir: &Path, _protocol: Protocol<'_>, _held: bool) -> String {
    "claim protocol not implemented".into()
}

#[cfg(test)]
mod tests;
