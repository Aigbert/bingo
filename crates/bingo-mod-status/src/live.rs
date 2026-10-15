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
                    tick(&host, &selections, std::time::Instant::now()).await;
                }
            }
        }
    });
}

async fn tick(host: &HostHandle, selections: &Selections, now: std::time::Instant) {
    let sessions = selections
        .lock()
        .map(|mut selections| {
            selections.retain(|_, selection| {
                now.saturating_duration_since(selection.seen) < std::time::Duration::from_secs(5)
            });
            selections.keys().cloned().collect::<Vec<_>>()
        })
        .unwrap_or_default();
    for session in sessions {
        let _ = host.invalidate(&session, vec![REQUEST.into()]).await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[tokio::test]
    async fn a_viewed_card_keeps_its_cursor_and_gets_a_tick() {
        let host = testing::TestHost::default();
        let selections = Selections::default();
        let now = std::time::Instant::now();
        let id = SessionId::from_raw("viewed");
        let mut selection: crate::Selection = "plugins".into();
        selection.seen = now;
        selections.lock().unwrap().insert(id.clone(), selection);
        tick(&host.handle(), &selections, now).await;
        assert!(
            matches!(host.calls().as_slice(), [testing::HostCall::Invalidate { session, .. }] if session == &id)
        );
        assert_eq!(selections.lock().unwrap()[&id].label, "plugins");
    }
    #[tokio::test]
    async fn unviewed_cards_stop_ticking_and_drop_their_cursor() {
        let host = testing::TestHost::default();
        let selections = Selections::default();
        let now = std::time::Instant::now();
        let mut selection: crate::Selection = "plugins".into();
        selection.seen = now;
        let id = SessionId::from_raw("unviewed");
        selections.lock().unwrap().insert(id.clone(), selection);
        tick(
            &host.handle(),
            &selections,
            now + std::time::Duration::from_secs(10),
        )
        .await;
        assert!(host.calls().is_empty());
        assert!(!selections.lock().unwrap().contains_key(&id));
    }
    #[tokio::test]
    async fn rendering_an_expired_card_restores_its_cursor_and_live_ticks() {
        use serde_json::json;
        let state = crate::tests::state();
        let id = state.summary.id.clone();
        let host = testing::TestHost::default();
        host.on_catalog(|kind| async move {
            Ok(Catalog {
                kind,
                entries: vec![],
            })
        });
        let selections = Selections::default();
        let now = std::time::Instant::now();
        selections.lock().unwrap().insert(
            id.clone(),
            crate::Selection {
                label: "model".into(),
                seen: now,
            },
        );
        tick(
            &host.handle(),
            &selections,
            now + std::time::Duration::from_secs(6),
        )
        .await;
        assert!(selections.lock().unwrap().is_empty());
        let drawing = crate::draw::Drawing {
            selections: selections.clone(),
            started: now,
        };
        let cx = HookContext {
            session: id.clone(),
            turn: None,
            item: None,
            cwd: "/unused".into(),
            provider: None,
            model: None,
            host: host.handle(),
        };
        let mut props = component::StatusProps::from(&state);
        props.selected = Some("model".into());
        let input = json!({"component":"Status","requestId":"status","surface":"desktop",
            "props":props,"viewport":{"columns":80,"rows":12}});
        let answer = drawing
            .chain(
                catalogue::UI_RENDER,
                &cx,
                input,
                Next::new(|_| Box::pin(async { Ok(json!(View::text("fallback"))) })),
            )
            .await
            .unwrap();
        assert!(
            serde_json::from_value::<View>(answer)
                .unwrap()
                .fold()
                .contains("❯ model")
        );
        let seen = selections.lock().unwrap()[&id].seen;
        tick(
            &host.handle(),
            &selections,
            seen + std::time::Duration::from_secs(1),
        )
        .await;
        assert!(host.calls().iter().any(|call| matches!(call,
            testing::HostCall::Invalidate { session, .. } if session == &id)));
        assert_eq!(selections.lock().unwrap()[&id].label, "model");
    }
}
