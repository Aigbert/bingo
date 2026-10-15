use bingo_sdk::*;
use serde_json::Value;

pub struct Lifecycle(pub crate::Shared);
#[async_trait::async_trait]
impl Interceptor for Lifecycle {
    fn id(&self) -> &str {
        "memory:lifecycle"
    }
    fn matcher(&self) -> LinkMatcher {
        LinkMatcher::at(catalogue::SESSION_END)
    }
    async fn observe(&self, _: &catalogue::Entry, cx: &HookContext, _: &Value) {
        if let Ok(mut cursors) = self.0.lock() {
            cursors.remove(&cx.session);
        }
    }
}

pub struct Disable(pub crate::Shared);
#[async_trait::async_trait]
impl Interceptor for Disable {
    fn id(&self) -> &str {
        "memory:disable"
    }
    fn matcher(&self) -> LinkMatcher {
        LinkMatcher {
            prefilter: serde_json::json!({"command":"plugins","args":"disable bingo.mod.memory"}),
            ..LinkMatcher::at(catalogue::COMMAND_RUN)
        }
    }
    async fn chain(
        &self,
        _: &catalogue::Entry,
        cx: &HookContext,
        value: Value,
        next: Next<'_, Value, Value>,
    ) -> Result<Value, KernelError> {
        let reply = next.run(value).await?;
        let sessions = {
            let mut cursors = self.0.lock().map_err(crate::error)?;
            std::mem::take(&mut *cursors)
                .into_keys()
                .collect::<Vec<_>>()
        };
        for session in sessions {
            cx.host
                .signal(&session, crate::ID, crate::REQUEST, Value::Null)
                .await?;
        }
        Ok(reply)
    }
}
