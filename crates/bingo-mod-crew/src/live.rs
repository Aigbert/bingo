use bingo_sdk::*;
use std::sync::{Arc, Mutex};
pub struct Follow {
    pub state: Arc<Mutex<SessionState>>,
    cancel: CancellationToken,
    _handle: SessionHandle,
}
impl Drop for Follow {
    fn drop(&mut self) {
        self.cancel.cancel();
    }
}
pub async fn follow(
    host: &HostHandle,
    owner: &SessionId,
    id: &SessionId,
) -> Result<Arc<Follow>, KernelError> {
    let attached = attach(host, id).await?;
    let mut snapshot = attached.snapshot;
    if snapshot.transcript.items().is_empty() {
        let page = attached
            .handle
            .history(HistoryPage {
                limit: 32,
                max_bytes: Some(32 * 1024),
                ..Default::default()
            })
            .await?;
        snapshot.transcript = page.items.into_iter().collect();
    }
    let events = attached.handle.events_since(snapshot.seq).await?;
    let follow = Arc::new(Follow {
        state: Arc::new(Mutex::new(snapshot)),
        cancel: CancellationToken::new(),
        _handle: attached.handle,
    });
    let (host, owner, state, cancel) = (
        host.clone(),
        owner.clone(),
        follow.state.clone(),
        follow.cancel.clone(),
    );
    tokio::spawn(async move {
        watch(host, owner, state, cancel, events).await;
    });
    Ok(follow)
}
async fn watch(
    host: HostHandle,
    owner: SessionId,
    state: Arc<Mutex<SessionState>>,
    cancel: CancellationToken,
    mut events: FrameStream,
) {
    loop {
        let frame = tokio::select! {
            _ = cancel.cancelled() => break,
            frame = std::future::poll_fn(|cx| events.as_mut().poll_next(cx)) => frame,
        };
        let Some(frame) = frame else { break };
        let changed = state
            .lock()
            .is_ok_and(|mut state| apply(&mut state, &frame));
        if changed && !cancel.is_cancelled() {
            let _ = host.invalidate(&owner, vec![crate::REQUEST.into()]).await;
        }
    }
}
pub async fn attach(host: &HostHandle, id: &SessionId) -> Result<Attachment, KernelError> {
    host.open(
        SessionSelector::ById { id: id.clone() },
        ClientIdentity {
            name: crate::ID.into(),
            // The attachment serves a terminal pane; identity remains the mod,
            // and the host stamps plugin provenance independently of this label.
            surface: "terminal".into(),
        },
        OpenOptions {
            max_snapshot_bytes: Some(64 * 1024),
            ..Default::default()
        },
    )
    .await
}

pub(crate) fn apply(state: &mut SessionState, frame: &Frame) -> bool {
    let before = card(state);
    let usage = state.summary.usage;
    state.apply(frame);
    usage != state.summary.usage || before != card(state)
}
fn card(state: &SessionState) -> crate::cards::Card {
    let row = bingo_services::agents_control::AgentRow {
        summary: state.summary.clone(),
    };
    crate::cards::agent_card(&row, Some(state), None)
}
