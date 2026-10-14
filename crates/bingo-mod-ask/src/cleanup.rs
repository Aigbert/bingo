use crate::States;
use bingo_sdk::*;

pub struct Cleanup(pub States);
#[async_trait::async_trait]
impl Hook for Cleanup {
    fn id(&self) -> &str { "ask.cleanup" }
    fn matcher(&self) -> HookMatcher { HookMatcher { points: vec![HookPoint::Event], tool: None } }
    async fn on_event(&self, frame: &Frame, _: &HookContext) {
        let Ok(mut states) = self.0.lock() else { return; };
        match &frame.event {
            Event::InteractionResolved { id, .. } | Event::InteractionCancelled { id, .. } => { states.remove(&(frame.session.clone(), id.to_string())); }
            Event::SessionClosed { .. } => states.retain(|(session, _), _| session != &frame.session),
            _ => {}
        }
    }
}
