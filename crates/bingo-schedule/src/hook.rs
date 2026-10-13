//! A journal cannot promise a wake this process no longer owns.
use crate::wakes::Wakes;
use async_trait::async_trait;
use bingo_sdk::{Hook, HookContext, HookMatcher, HookPoint, Phase};
use std::sync::Arc;

#[derive(Debug)]
pub struct ScheduleHook(pub Arc<Wakes>);

#[async_trait]
impl Hook for ScheduleHook {
    fn id(&self) -> &str {
        "schedule"
    }
    fn matcher(&self) -> HookMatcher {
        HookMatcher {
            points: vec![HookPoint::Session],
            tool: None,
        }
    }
    async fn on_session(&self, phase: Phase, cx: &HookContext) {
        if phase == Phase::Start && self.0.pending(&cx.session).is_none() {
            self.0.clear_stale(cx.host.clone(), cx.session.clone());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{tests::Fixture, wake};
    use bingo_sdk::SessionId;
    use jiff::{SignedDuration, Timestamp};
    use serde_json::Value;

    #[tokio::test]
    async fn a_new_process_clears_the_dead_wake_and_its_status_slot() {
        let fixture = Fixture::new();
        let session = SessionId::from_raw("ses_woken");
        let pending = wake::set(
            Timestamp::now(),
            SignedDuration::from_secs(60),
            "check".into(),
        );
        let first = ScheduleHook(Arc::default());
        first.0.set(&session, pending.clone());
        wake::publish(&fixture.handle(), &session, Some(&pending)).await;
        drop(first);
        let second = ScheduleHook(Arc::default());
        let cx = HookContext {
            host: fixture.handle(),
            session: session.clone(),
            turn: None,
            cwd: fixture.cwd(),
            provider: None,
            model: None,
        };
        second.on_session(Phase::Start, &cx).await;
        for _ in 0..16 {
            tokio::task::yield_now().await;
        }
        let writes = fixture.host.extended();
        assert!(writes[0].3["slots"].is_array());
        assert_eq!(writes.last().unwrap().3, Value::Null);
        second.0.set(&session, pending);
        second.on_session(Phase::Start, &cx).await;
        assert_eq!(
            fixture.host.extended().len(),
            writes.len(),
            "a held wake survives a session start"
        );
    }
}
