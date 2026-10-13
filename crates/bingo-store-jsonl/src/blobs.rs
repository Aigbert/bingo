//! Content-addressed picture bytes, owned by one session directory.

use bingo_sdk::KernelError;
use std::io::Write;
use std::path::Path;

pub(crate) fn put(dir: &Path, bytes: &[u8]) -> Result<String, KernelError> {
    let hash = bingo_digest::hex(&bingo_digest::digest(bytes));
    let blobs = dir.join("blobs");
    let destination = blobs.join(&hash);
    if destination.is_file() {
        return Ok(hash);
    }
    std::fs::create_dir_all(&blobs).map_err(io)?;
    let temporary = blobs.join(format!("{}.tmp", bingo_sdk::ItemId::mint()));
    let result = publish(&temporary, &destination, bytes);
    if result.is_err() {
        let _ = std::fs::remove_file(&temporary);
    }
    result?;
    Ok(hash)
}

fn publish(temporary: &Path, destination: &Path, bytes: &[u8]) -> Result<(), KernelError> {
    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(temporary)
        .map_err(io)?;
    file.write_all(bytes)
        .and_then(|()| file.flush())
        .map_err(io)?;
    drop(file);
    match std::fs::rename(temporary, destination) {
        Ok(()) => Ok(()),
        // Windows may refuse replacing a blob another writer just published.
        Err(_) if destination.is_file() => std::fs::remove_file(temporary).map_err(io),
        Err(error) => Err(io(error)),
    }
}

pub(crate) fn get(dir: &Path, hash: &str) -> Option<Vec<u8>> {
    if hash.len() != 64 || !hash.bytes().all(|b| b.is_ascii_hexdigit()) {
        return None;
    }
    let bytes = std::fs::read(dir.join("blobs").join(hash)).ok()?;
    (bingo_digest::hex(&bingo_digest::digest(&bytes)) == hash).then_some(bytes)
}

fn io(error: std::io::Error) -> KernelError {
    crate::storage(format!("picture blob: {error}"))
}
