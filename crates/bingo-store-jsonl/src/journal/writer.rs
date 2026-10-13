//! Prepare each open journal once; subsequent appends do not scan its history.

use bingo_sdk::{Frame, KernelError};
use std::collections::HashSet;
use std::path::{Path, PathBuf};

#[derive(Debug, Default)]
pub(crate) struct Writer {
    prepared: HashSet<PathBuf>,
}

impl Writer {
    pub(crate) fn append(&mut self, dir: &Path, frame: &Frame) -> Result<(), KernelError> {
        self.prepare(dir)?;
        super::append_prepared(dir, frame)
    }

    pub(crate) fn prepare(&mut self, dir: &Path) -> Result<(), KernelError> {
        if !self.prepared.contains(dir) {
            super::prepare(dir)?;
            self.resumed(dir);
        }
        Ok(())
    }

    /// A verified v2 checkpoint and its tail already checked the format.
    pub(crate) fn resumed(&mut self, dir: &Path) {
        self.prepared.insert(dir.to_path_buf());
    }

    pub(crate) fn forget(&mut self, dir: &Path) {
        self.prepared.remove(dir);
    }
}
