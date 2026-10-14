use crate::States;
use bingo_sdk::*;

pub struct Cleanup(pub States);
#[async_trait::async_trait]
impl Hook for Cleanup {
    fn id(&self) -> &str {
        "ask.cleanup"
    }
    fn matcher(&self) -> HookMatcher {
        HookMatcher {
            points: vec![HookPoint::Event],
            tool: None,
        }
    }
    async fn on_event(&self, frame: &Frame, _: &HookContext) {
        let Ok(mut states) = self.0.lock() else {
            return;
        };
        match &frame.event {
            Event::InteractionResolved { id, .. } | Event::InteractionCancelled { id, .. } => {
                states.remove(&(frame.session.clone(), id.to_string()));
            }
            Event::SessionClosed { .. } => {
                states.retain(|(session, _), _| session != &frame.session)
            }
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn closing_an_interaction_releases_only_its_state_without_another_key() {
        for event in [
            Event::InteractionResolved {
                id: InteractionId::from_raw("q"),
                answer: Answer::Cancel,
                by: ResolvedBy::Kernel,
            },
            Event::InteractionCancelled {
                id: InteractionId::from_raw("q"),
                reason: CancelReason::Interrupted,
            },
        ] {
            let states = States::default();
            let session = SessionId::from_raw("s");
            states
                .lock()
                .unwrap()
                .insert((session.clone(), "q".into()), Default::default());
            states
                .lock()
                .unwrap()
                .insert((session.clone(), "other".into()), Default::default());
            let frame: Frame = serde_json::from_value(serde_json::json!({"session":"s","seq":1,"ts":"2026-10-15T00:00:00Z","event":event})).unwrap();
            let cx = HookContext {
                session: session.clone(),
                turn: None,
                item: None,
                cwd: "/unused".into(),
                provider: None,
                model: None,
                host: bingo_sdk::testing::NoHost::handle(),
            };
            let cleanup = Cleanup(states.clone());
            let mut future = std::pin::pin!(cleanup.on_event(&frame, &cx));
            assert!(matches!(
                std::future::Future::poll(
                    future.as_mut(),
                    &mut std::task::Context::from_waker(std::task::Waker::noop())
                ),
                std::task::Poll::Ready(())
            ));
            assert!(
                !states
                    .lock()
                    .unwrap()
                    .contains_key(&(session.clone(), "q".into()))
            );
            assert!(
                states
                    .lock()
                    .unwrap()
                    .contains_key(&(session, "other".into()))
            );
        }
    }
}
