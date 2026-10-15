use super::*;
use tokio::sync::oneshot;

pub(super) async fn delete(host: &Host, root: &SessionId) -> Result<(), KernelError> {
    // Reopen cannot turn a stored reservation into an actor while we own it.
    let _guard = host.reopening.lock().await;
    let summaries = host.sessions(SessionFilter::default()).await?;
    let mut ids = Vec::new();
    collect(root, &summaries, &mut ids);
    let mut claims = Claims {
        store: host.registry.store.clone(),
        stored: Vec::new(),
        live: Vec::new(),
    };
    for id in &ids {
        if let Err(error) = reserve(host, id, &mut claims).await {
            if let Some(store) = &claims.store {
                for id in &claims.stored {
                    let _ = store.release(id).await;
                }
            }
            claims.stored.clear();
            return Err(error);
        }
    }
    for commit in claims.live.drain(..) {
        let _ = commit.send(());
    }
    for id in ids {
        host.delete_one(&id).await?;
    }
    claims.stored.clear();
    Ok(())
}

fn collect(root: &SessionId, all: &[SessionSummary], out: &mut Vec<SessionId>) {
    let mut frontier = vec![root.clone()];
    let mut visited = std::collections::BTreeSet::new();
    while let Some(id) = frontier.pop() {
        if !visited.insert(id.clone()) {
            continue;
        }
        frontier.extend(
            all.iter()
                .filter(|s| s.parent.as_ref().is_some_and(|p| p.session == id))
                .map(|s| s.id.clone()),
        );
        out.push(id);
    }
    out.reverse();
}

async fn reserve(host: &Host, id: &SessionId, claims: &mut Claims) -> Result<(), KernelError> {
    if let Ok(live) = host.live(id) {
        claims.live.push(live.mailbox.reserve_delete().await?);
    } else if let Some(store) = &claims.store {
        store.acquire(id).await?;
        claims.stored.push(id.clone());
    } else {
        return Err(KernelError::new(
            ErrorCode::SessionNotFound,
            "no such session",
        ));
    }
    Ok(())
}

struct Claims {
    store: Option<Arc<dyn SessionStore>>,
    stored: Vec<SessionId>,
    live: Vec<oneshot::Sender<()>>,
}
impl Drop for Claims {
    fn drop(&mut self) {
        let Some(store) = self.store.clone() else {
            return;
        };
        let ids = std::mem::take(&mut self.stored);
        tokio::spawn(async move {
            for id in ids {
                let _ = store.release(&id).await;
            }
        });
    }
}
