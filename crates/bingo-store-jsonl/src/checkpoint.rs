//! Verified checkpoints beside the append-only journal.

mod witness;

use std::path::Path;

use bingo_sdk::store::{CHECKPOINT_FORMAT, CHECKPOINT_VERSION};
use bingo_sdk::{Checkpoint, Event, KernelError, Resumed, Seq, SessionId, SessionState};

use crate::{journal, storage};

pub(crate) fn write(
    dir: &Path,
    session: &SessionId,
    checkpoint: &Checkpoint,
) -> Result<(), KernelError> {
    let mut checkpoint = checkpoint.clone();
    check_identity(&checkpoint, session)?;
    if let Ok(existing) = read(dir, session)
        && existing.seq > checkpoint.seq
    {
        return Ok(());
    }
    let (offset, line) = witness::locate(dir, session, checkpoint.seq)?;
    checkpoint.offset = offset;
    checkpoint.line_hash = bingo_digest::hex(&bingo_digest::digest(&line));
    let bytes =
        serde_json::to_vec(&checkpoint).map_err(|e| storage(format!("encode checkpoint: {e}")))?;
    let temporary = dir.join("checkpoint.json.tmp");
    std::fs::write(&temporary, bytes)
        .and_then(|()| std::fs::rename(&temporary, dir.join("checkpoint.json")))
        .map_err(|e| storage(format!("write checkpoint: {e}")))
}

fn check_identity(checkpoint: &Checkpoint, session: &SessionId) -> Result<(), KernelError> {
    if checkpoint.format != CHECKPOINT_FORMAT || checkpoint.version != CHECKPOINT_VERSION {
        return Err(storage("checkpoint format/version mismatch"));
    }
    if checkpoint.state.summary.id != *session || checkpoint.state.seq != checkpoint.seq {
        return Err(storage("checkpoint session/sequence mismatch"));
    }
    Ok(())
}

fn read(dir: &Path, session: &SessionId) -> Result<Checkpoint, KernelError> {
    let bytes = std::fs::read(dir.join("checkpoint.json"))
        .map_err(|e| storage(format!("read checkpoint: {e}")))?;
    let checkpoint: Checkpoint =
        serde_json::from_slice(&bytes).map_err(|e| storage(format!("decode checkpoint: {e}")))?;
    check_identity(&checkpoint, session)?;
    let line = witness::at(dir, checkpoint.offset)?;
    if bingo_digest::hex(&bingo_digest::digest(&line)) != checkpoint.line_hash {
        return Err(storage("checkpoint line hash mismatch"));
    }
    let frame: bingo_sdk::Frame = serde_json::from_slice(&line)
        .map_err(|e| storage(format!("checkpoint line is not a frame: {e}")))?;
    if frame.seq != checkpoint.seq || frame.session != *session {
        return Err(storage("checkpoint line session/sequence mismatch"));
    }
    Ok(checkpoint)
}

pub(crate) fn resume(dir: &Path, session: &SessionId) -> Result<Resumed, KernelError> {
    match read(dir, session) {
        Ok(checkpoint) => {
            let tail = journal::tail(dir, checkpoint.offset)?;
            Ok(Resumed {
                state: Some(checkpoint.state),
                tail,
            })
        }
        Err(error) => {
            tracing::warn!(%session, %error, "checkpoint rejected; replaying journal");
            let tail = journal::replay(dir, Seq::ZERO)?;
            repair(dir, session, &tail);
            Ok(Resumed { state: None, tail })
        }
    }
}

fn repair(dir: &Path, session: &SessionId, frames: &[bingo_sdk::Frame]) {
    let Some(bingo_sdk::Frame {
        event: Event::SessionUpdated { summary },
        ..
    }) = frames.first()
    else {
        return;
    };
    let mut state = SessionState::new(summary.clone());
    for frame in frames {
        state.apply(frame);
    }
    state.transcript.drop_before_cut();
    if let Err(error) = write(dir, session, &Checkpoint::new(state)) {
        tracing::warn!(%session, %error, "checkpoint repair failed");
    }
}

#[cfg(test)]
mod tests;
