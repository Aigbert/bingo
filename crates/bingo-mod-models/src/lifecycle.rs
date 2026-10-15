use bingo_sdk::*;
pub struct Lifecycle(pub crate::Shared);
#[async_trait::async_trait]
impl Hook for Lifecycle {
    fn id(&self) -> &str {
        "models.lifecycle"
    }
    fn matcher(&self) -> HookMatcher {
        HookMatcher {
            points: vec![HookPoint::Event],
            tool: None,
        }
    }
    async fn on_event(&self, frame: &Frame, cx: &HookContext) {
        if matches!(frame.event, Event::SessionClosed { .. }) {
            if let Ok(mut states) = self.0.lock() {
                states.remove(&frame.session);
            }
            return;
        }
        if !matches!(
            frame.event,
            Event::CatalogChanged { .. } | Event::SessionUpdated { .. } | Event::TurnUsage { .. }
        ) {
            return;
        }
        let id = self
            .0
            .lock()
            .ok()
            .and_then(|states| states.get(&frame.session).map(|p| p.id.clone()));
        if let Some(id) = id {
            let _ = cx.host.invalidate(&frame.session, vec![id]).await;
        }
    }
}
