use bingo_sdk::*;
use std::collections::BTreeSet;

pub struct Refresh(pub crate::Shared);
#[async_trait::async_trait]
impl Hook for Refresh {
    fn id(&self) -> &str {
        "crew.refresh"
    }
    fn matcher(&self) -> HookMatcher {
        HookMatcher {
            points: vec![HookPoint::Event],
            tool: None,
        }
    }
    async fn on_event(&self, frame: &Frame, cx: &HookContext) {
        let sessions = self
            .0
            .lock()
            .map(|mut panels| {
                if matches!(frame.event, Event::SessionClosed { .. }) {
                    panels.remove(&frame.session);
                }
                panels
                    .iter()
                    .filter(|(_, watching)| relevant(watching, frame))
                    .map(|(id, _)| id.clone())
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();
        for session in sessions {
            let _ = cx
                .host
                .invalidate(&session, vec![crate::REQUEST.into()])
                .await;
        }
    }
}
/// Whether a frame changes what a sheet watching these sessions draws.
pub(crate) fn relevant(watching: &BTreeSet<SessionId>, frame: &Frame) -> bool {
    match &frame.event {
        Event::SessionUpdated { summary } => {
            watching.contains(&summary.id)
                || summary
                    .parent
                    .as_ref()
                    .is_some_and(|p| watching.contains(&p.session))
        }
        Event::SessionClosed { .. } => watching.contains(&frame.session),
        Event::Extension { plugin, .. } => {
            watching.contains(&frame.session) && plugin == "bingo.rooms"
        }
        Event::TurnStarted { .. } | Event::ItemCompleted { .. } => {
            watching.contains(&frame.session)
        }
        _ => false,
    }
}
