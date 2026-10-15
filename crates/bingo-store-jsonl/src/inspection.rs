//! Stored inspection and title changes never start a session actor.
use crate::*;

pub(super) fn summary(store: &JsonlStore, id: &SessionId) -> Result<SessionSummary, KernelError> {
    let dir = store.existing(id)?;
    let mut summary = match summary::read(&dir) {
        Some(summary) => summary,
        None => folded(&dir)?.summary,
    };
    let modified = std::fs::metadata(layout::journal(&dir))
        .and_then(|metadata| metadata.modified())
        .map_err(|e| storage(e.to_string()))?;
    summary.updated_at = Timestamp::try_from(modified).map_err(|e| storage(e.to_string()))?;
    Ok(summary)
}

fn folded(dir: &Path) -> Result<bingo_sdk::SessionState, KernelError> {
    let frames = journal::replay(dir, Seq::ZERO)?;
    let summary = frames
        .iter()
        .find_map(|frame| match &frame.event {
            Event::SessionUpdated { summary } => Some(summary.clone()),
            _ => None,
        })
        .ok_or_else(|| storage("journal has no summary"))?;
    let mut state = bingo_sdk::SessionState::new(summary);
    for frame in frames {
        state.apply(&frame);
    }
    Ok(state)
}

pub(super) fn rename(store: &JsonlStore, id: &SessionId, title: &str) -> Result<(), KernelError> {
    let dir = store.existing(id)?;
    let _claim = lock::take(&layout::lock(&dir))?.ok_or_else(|| {
        KernelError::new(
            ErrorCode::SessionLocked,
            "session is open elsewhere; close it before renaming",
        )
    })?;
    let file = std::fs::OpenOptions::new()
        .write(true)
        .open(layout::journal(&dir))
        .map_err(|e| storage(e.to_string()))?;
    let modified = file
        .metadata()
        .and_then(|metadata| metadata.modified())
        .map_err(|e| storage(e.to_string()))?;
    let mut state = folded(&dir)?;
    state.summary.title = Some(title.to_owned());
    state.summary.updated_at = Timestamp::try_from(modified).map_err(|e| storage(e.to_string()))?;
    let frame = Frame {
        seq: Seq(state.seq.0 + 1),
        ts: state.summary.updated_at,
        session: id.clone(),
        cause: None,
        event: Event::SessionUpdated {
            summary: state.summary,
        },
    };
    let result = store.append_frame(id, &frame);
    // List activity is the journal mtime, including for older summaries.
    let restored = file
        .set_modified(modified)
        .map_err(|e| storage(e.to_string()));
    store
        .writer
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .forget(&dir);
    result.and(restored)
}
