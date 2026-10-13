//! Find just the line anchoring a checkpoint, without loading the prefix.

use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::path::Path;

use crate::{layout, storage};
use bingo_sdk::{Frame, KernelError, Seq, SessionId};

fn io(error: std::io::Error) -> KernelError {
    storage(format!("checkpoint journal witness: {error}"))
}

pub(super) fn at(dir: &Path, offset: u64) -> Result<Vec<u8>, KernelError> {
    let mut file = File::open(layout::journal(dir)).map_err(io)?;
    let (_, line) = before(&mut file, offset)?;
    Ok(line)
}

pub(super) fn locate(
    dir: &Path,
    session: &SessionId,
    seq: Seq,
) -> Result<(u64, Vec<u8>), KernelError> {
    let mut file = File::open(layout::journal(dir)).map_err(io)?;
    let length = file.metadata().map_err(io)?.len();
    // A torn suffix is not a frame and cannot anchor a repaired checkpoint.
    let mut end = previous_newline(&mut file, length)?.map_or(0, |at| at + 1);
    while end > 0 {
        let (start, line) = before(&mut file, end)?;
        if let Ok(frame) = serde_json::from_slice::<Frame>(&line) {
            if frame.seq == seq && frame.session == *session {
                return Ok((end, line));
            }
            if frame.seq < seq {
                break;
            }
        }
        end = start;
    }
    Err(storage("checkpoint sequence is absent from journal"))
}

fn before(file: &mut File, end: u64) -> Result<(u64, Vec<u8>), KernelError> {
    if end == 0 || end > file.metadata().map_err(io)?.len() {
        return Err(storage("checkpoint offset outside journal"));
    }
    file.seek(SeekFrom::Start(end - 1)).map_err(io)?;
    let mut byte = [0];
    file.read_exact(&mut byte).map_err(io)?;
    if byte[0] != b'\n' {
        return Err(storage("checkpoint offset is not after a complete line"));
    }
    let start = previous_newline(file, end - 1)?.map_or(0, |at| at + 1);
    file.seek(SeekFrom::Start(start)).map_err(io)?;
    let mut line = Vec::new();
    file.take(end - start).read_to_end(&mut line).map_err(io)?;
    Ok((start, line))
}

fn previous_newline(file: &mut File, mut end: u64) -> Result<Option<u64>, KernelError> {
    let mut block = [0; 8192];
    while end > 0 {
        let start = end.saturating_sub(block.len() as u64);
        let length = (end - start) as usize;
        file.seek(SeekFrom::Start(start)).map_err(io)?;
        file.read_exact(&mut block[..length]).map_err(io)?;
        if let Some(at) = block[..length].iter().rposition(|b| *b == b'\n') {
            return Ok(Some(start + at as u64));
        }
        end = start;
    }
    Ok(None)
}
