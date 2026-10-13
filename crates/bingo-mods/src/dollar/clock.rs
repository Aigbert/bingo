//! Timers belong to a session and to one callback generation.
use bingo_plugin_host::callback::Executor;
use bingo_sdk::*;
use std::{sync::Arc, time::Duration};

#[derive(Default)]
pub struct Timers;
impl Timers {
    pub fn start(
        &self,
        _: HookContext,
        _: Arc<dyn Executor>,
        _: String,
        _: Duration,
        _: bool,
    ) -> String {
        "unavailable".into()
    }
    pub async fn end(&self, _: &SessionId) {}
    pub async fn stop(&self) {}
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::Value;
    struct Counter {
        stop: CancellationToken,
        calls: tokio::sync::mpsc::UnboundedSender<SessionId>,
    }
    #[async_trait::async_trait]
    impl Executor for Counter {
        fn stopped(&self) -> CancellationToken {
            self.stop.clone()
        }
        async fn prepare(&self) -> Result<(), String> {
            Ok(())
        }
        async fn call(&self, _: &str, _: Value, cx: &HookContext) -> Result<Value, String> {
            assert!(cx.turn.is_none());
            self.calls.send(cx.session.clone()).unwrap();
            Ok(Value::Null)
        }
    }
    fn context(session: &str) -> HookContext {
        HookContext {
            session: SessionId::from_raw(session),
            turn: None,
            item: None,
            cwd: ".".into(),
            provider: None,
            model: None,
            host: testing::NoHost::handle(),
        }
    }
    async fn fired(calls: &mut tokio::sync::mpsc::UnboundedReceiver<SessionId>) -> SessionId {
        tokio::time::timeout(Duration::from_millis(1), calls.recv())
            .await
            .expect("timer did not fire")
            .unwrap()
    }
    #[tokio::test(start_paused = true)]
    async fn periods_are_exact_and_generation_end_and_session_end_cancel_timers() {
        let timers = Timers;
        let (tx, mut calls) = tokio::sync::mpsc::unbounded_channel();
        let stop = CancellationToken::new();
        let executor = Arc::new(Counter {
            stop: stop.clone(),
            calls: tx,
        });
        timers.start(
            context("one"),
            executor.clone(),
            "tick".into(),
            Duration::from_secs(1),
            true,
        );
        tokio::task::yield_now().await;
        tokio::time::advance(Duration::from_millis(999)).await;
        assert!(calls.try_recv().is_err());
        tokio::time::advance(Duration::from_millis(1)).await;
        assert_eq!(fired(&mut calls).await, SessionId::from_raw("one"));
        tokio::time::advance(Duration::from_secs(1)).await;
        assert_eq!(fired(&mut calls).await, SessionId::from_raw("one"));
        timers.end(&SessionId::from_raw("one")).await;
        timers.start(
            context("two"),
            executor.clone(),
            "once".into(),
            Duration::from_secs(1),
            false,
        );
        tokio::task::yield_now().await;
        tokio::time::advance(Duration::from_secs(1)).await;
        assert_eq!(fired(&mut calls).await, SessionId::from_raw("two"));
        timers.start(
            context("three"),
            executor,
            "tick".into(),
            Duration::from_secs(1),
            true,
        );
        tokio::task::yield_now().await;
        stop.cancel();
        tokio::time::advance(Duration::from_secs(10)).await;
        timers.stop().await;
        assert!(calls.try_recv().is_err());
    }
}
