use super::*;

pub(super) fn store(host: &Host) -> Result<&dyn SessionStore, KernelError> {
    host.registry
        .store
        .as_deref()
        .ok_or_else(|| KernelError::new(ErrorCode::NotReady, "operation needs a session store"))
}

pub(super) async fn summary(host: &Host, id: &SessionId) -> Result<SessionSummary, KernelError> {
    if let Ok(live) = host.live(id) {
        return live.mailbox.summary().await;
    }
    store(host)?.summary(id).await
}

pub(super) async fn read(host: &Host, id: &SessionId) -> Result<SessionState, KernelError> {
    if let Ok(live) = host.live(id) {
        return live.mailbox.attach().await.map(|(state, _)| state);
    }
    let store = store(host)?;
    let summary = store.summary(id).await?;
    let mut state = SessionState::new(summary.clone());
    for frame in store.replay(id, Seq::ZERO).await? {
        state.apply(&frame);
    }
    state.summary.updated_at = summary.updated_at;
    Ok(state)
}

/// The store's lock answers for every session this host has not opened.
pub(super) async fn open_elsewhere(host: &Host, id: &SessionId) -> Result<bool, KernelError> {
    if host.live(id).is_ok() {
        return Ok(false);
    }
    store(host)?.locked(id).await
}

pub(super) async fn check_delete(host: &Host, id: &SessionId) -> Result<(), KernelError> {
    if let Ok(live) = host.live(id) {
        if live.mailbox.summary().await?.busy {
            return Err(KernelError::new(
                ErrorCode::NotReady,
                "session is busy; wait for its turn to finish",
            ));
        }
    } else if store(host)?.locked(id).await? {
        return Err(KernelError::new(
            ErrorCode::SessionLocked,
            "session is open elsewhere; close it before deleting",
        ));
    }
    Ok(())
}
