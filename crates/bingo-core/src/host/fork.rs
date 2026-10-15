use super::*;

pub(super) async fn fork_session(
    host: &Host,
    source: &SessionId,
    at: Option<TurnId>,
) -> Result<SessionId, KernelError> {
    let store = host
        .registry
        .store
        .clone()
        .ok_or_else(|| KernelError::new(ErrorCode::NotReady, "fork needs a session store"))?;
    let (frames, at) = source_frames(host, store.as_ref(), source, at).await?;
    let mut frames = crate::fork::kept(&frames, at.as_ref())?;
    let link = ForkLink {
        session: source.clone(),
        turn: at,
    };
    let id = SessionId::mint();
    crate::fork::rewrite(&mut frames, &id, &link);
    let mut summary = session::fold_resumed(&Resumed {
        state: None,
        tail: frames.clone(),
    })?
    .summary;
    crate::fork::root(&mut summary, &id, &link);
    summary.created_at = Timestamp::now();
    summary.updated_at = summary.created_at;
    store.create(&summary).await?;
    let mut journal = ForkJournal {
        store: store.clone(),
        id: id.clone(),
        committed: false,
    };
    store.acquire(&id).await?;
    if let Err(error) = append_fork(store.as_ref(), &id, frames, summary).await {
        let _ = store.delete(&id).await;
        journal.committed = true;
        return Err(error);
    }
    store.release(&id).await?;
    journal.committed = true;
    Ok(id)
}

async fn append_fork(
    store: &dyn SessionStore,
    id: &SessionId,
    frames: Vec<Frame>,
    summary: SessionSummary,
) -> Result<(), KernelError> {
    let seq = frames.last().map_or(Seq(1), |frame| Seq(frame.seq.0 + 1));
    for frame in frames {
        store.append(id, &frame).await?;
    }
    store
        .append(
            id,
            &Frame {
                seq,
                ts: summary.updated_at,
                session: id.clone(),
                cause: None,
                event: Event::SessionUpdated { summary },
            },
        )
        .await
}

/// Cancellation drops the unfinished copy as well as the caller's future.
struct ForkJournal {
    store: Arc<dyn SessionStore>,
    id: SessionId,
    committed: bool,
}
impl Drop for ForkJournal {
    fn drop(&mut self) {
        if self.committed {
            return;
        }
        let store = self.store.clone();
        let id = self.id.clone();
        tokio::spawn(async move {
            let _ = store.delete(&id).await;
        });
    }
}

fn forkable(state: &SessionState) -> Result<(), KernelError> {
    if state.busy() {
        return Err(KernelError::new(
            ErrorCode::NotReady,
            "cannot fork while a turn is running",
        ));
    }
    if state.summary.parent.is_some() || state.summary.driver == Driver::Log {
        return Err(KernelError::new(
            ErrorCode::InvalidInput,
            "only root model sessions can be forked",
        ));
    }
    Ok(())
}

async fn source_frames(
    host: &Host,
    store: &dyn SessionStore,
    source: &SessionId,
    at: Option<TurnId>,
) -> Result<(Vec<Frame>, Option<TurnId>), KernelError> {
    if let Ok(live) = host.live(source) {
        // The actor fixes the cut before any disk read.
        let (state, _, _) = live.mailbox.attach_bounded(Some(65536)).await?;
        forkable(&state)?;
        let frames = store
            .replay(source, Seq::ZERO)
            .await?
            .into_iter()
            .take_while(|frame| frame.seq <= state.seq)
            .collect();
        return Ok((frames, at));
    }
    let frames = store.replay(source, Seq::ZERO).await?;
    let mut state = session::fold_resumed(&Resumed {
        state: None,
        tail: frames.clone(),
    })?;
    if state.busy() && store.locked(source).await? {
        return Err(KernelError::new(
            ErrorCode::SessionLocked,
            "session is running elsewhere; wait for its turn to finish before forking",
        ));
    }
    // A crashed writer may have left an incomplete tail; only completed turns survive.
    state.turn = None;
    state.summary.busy = false;
    forkable(&state)?;
    let at = at
        .or_else(|| {
            frames.iter().rev().find_map(|frame| match &frame.event {
                Event::TurnCompleted { turn, .. } => Some(turn.clone()),
                _ => None,
            })
        })
        .ok_or_else(|| {
            KernelError::new(
                ErrorCode::InvalidInput,
                "session has no completed turn to fork",
            )
        })?;
    Ok((frames, Some(at)))
}
