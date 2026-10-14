use crate::{REQUEST, Selections};
use bingo_sdk::*;

pub struct Refresh(pub Selections);
#[async_trait::async_trait]
impl Hook for Refresh {
    fn id(&self) -> &str {
        "status.refresh"
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
            Event::ConfigChanged { .. }
                | Event::SessionUpdated { .. }
                | Event::TurnUsage { .. }
                | Event::UsageCharged { .. }
        ) {
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
// MCP status has no session frame. Re-ask open cards for uptime and live service
// facts; the event hook handles session changes without waiting for this tick.
pub fn start(host: HostHandle, selections: Selections, stop: CancellationToken) {
    tokio::spawn(async move {
        let mut ticks = tokio::time::interval(std::time::Duration::from_secs(1));
        loop {
            tokio::select! {
                _ = stop.cancelled() => break,
                _ = ticks.tick() => {
                    let sessions = selections.lock().map(|s| s.keys().cloned().collect::<Vec<_>>()).unwrap_or_default();
                    for session in sessions { let _ = host.invalidate(&session, vec![REQUEST.into()]).await; }
                }
            }
        }
    });
}
