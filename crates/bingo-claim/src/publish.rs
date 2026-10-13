//! Publish a complete marker without exposing a blank legacy-shaped file.

use std::fs::File;
use std::io::{self, Write};
use std::path::{Path, PathBuf};

use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

pub(super) fn open(path: &Path, marker: &str) -> io::Result<File> {
    match existing(path) {
        Ok(file) => Ok(file),
        Err(e) if e.kind() == io::ErrorKind::NotFound => {
            publish(path, marker)?;
            existing(path)
        }
        Err(e) => Err(e),
    }
}

fn existing(path: &Path) -> io::Result<File> {
    File::options().read(true).write(true).open(path)
}

fn publish(path: &Path, marker: &str) -> io::Result<()> {
    let (staging, mut file) = staging(path)?;
    let _cleanup = Staging(staging.clone());
    file.write_all(marker.as_bytes())?;
    file.sync_all()?;
    drop(file);
    // A hard link publishes complete bytes atomically without replacing an
    // inode a modern runner has locked or a legacy runner has claimed.
    match std::fs::hard_link(staging, path) {
        Ok(()) => Ok(()),
        Err(e) if e.kind() == io::ErrorKind::AlreadyExists => Ok(()),
        Err(e) => Err(e),
    }
}

struct Staging(PathBuf);

impl Drop for Staging {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.0);
    }
}

fn staging(path: &Path) -> io::Result<(PathBuf, File)> {
    loop {
        let id = NEXT.fetch_add(1, Ordering::Relaxed);
        let staging = path.with_file_name(format!(".claim-{}-{id}.tmp", std::process::id()));
        match File::create_new(&staging) {
            Ok(file) => return Ok((staging, file)),
            Err(error) if error.kind() == io::ErrorKind::AlreadyExists => continue,
            Err(error) => return Err(error),
        }
    }
}
