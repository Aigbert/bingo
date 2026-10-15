use crate::{REQUEST, Shared};
use bingo_sdk::*;

pub struct Refresh(pub Shared);
#[async_trait::async_trait]
impl Hook for Refresh {
    fn id(&self) -> &str {
        "schedules.refresh"
    }
    fn matcher(&self) -> HookMatcher {
        HookMatcher {
            points: vec![HookPoint::Event],
            tool: None,
        }
    }
    async fn on_event(&self, frame: &Frame, cx: &HookContext) {
        if matches!(frame.event, Event::SessionClosed { .. }) {
            if let Ok(mut selections) = self.0.lock() {
                selections.remove(&frame.session);
            }
            return;
        }
        if !matches!(
            frame.event,
            Event::IntentAck { .. }
                | Event::ConfigChanged { .. }
                | Event::SessionUpdated { .. }
                | Event::TurnUsage { .. }
                | Event::UsageCharged { .. }
        ) && !matches!(&frame.event, Event::Extension { plugin, .. } if plugin == "bingo.schedule")
        {
            return;
        }
        let open = self.0.lock().is_ok_and(|s| s.contains_key(&frame.session));
        if open {
            let _ = cx
                .host
                .invalidate(&frame.session, vec![REQUEST.into()])
                .await;
        }
    }
}
// Other processes and hand edits change the store without a local frame.
pub fn start(host: HostHandle, selections: Shared, stop: CancellationToken) {
    tokio::spawn(async move {
        let mut ticks = tokio::time::interval(std::time::Duration::from_secs(1));
        loop {
            tokio::select! {
                _ = stop.cancelled() => break,
                _ = ticks.tick() => {
                    tick(&host, &selections, std::time::Instant::now()).await;
                }
            }
        }
    });
}

async fn tick(host: &HostHandle, selections: &Shared, now: std::time::Instant) {
    let sessions = selections
        .lock()
        .map(|mut selections| {
            selections.retain(|_, selection| {
                selection.seen.is_some_and(|seen| {
                    now.saturating_duration_since(seen) < std::time::Duration::from_secs(5)
                })
            });
            selections.keys().cloned().collect::<Vec<_>>()
        })
        .unwrap_or_default();
    for session in sessions {
        let _ = host.invalidate(&session, vec![REQUEST.into()]).await;
    }
}
