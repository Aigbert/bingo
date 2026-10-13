//! A channel carries JavaScript effects; the host owns every continuation.
use crate::module::Module;
use async_trait::async_trait;
use bingo_sdk::*;
use serde_json::{Value, json};
use std::sync::Arc;

tokio::task_local! { static CALLING: Vec<(String, usize)>; }

pub(crate) struct Link(pub Arc<Module>);
#[async_trait]
impl Interceptor for Link {
    fn id(&self) -> &str {
        &self.0.found.name
    }
    fn matcher(&self) -> LinkMatcher {
        LinkMatcher {
            patterns: self
                .0
                .engine
                .as_ref()
                .into_iter()
                .flat_map(|engine| engine.patterns())
                .filter_map(|pattern| catalogue::Pattern::parse(pattern).ok())
                .collect(),
            prefilter: Value::Null,
        }
    }
    async fn chain(
        &self,
        entry: &catalogue::Entry,
        cx: &HookContext,
        input: Value,
        next: Next<'_, Value, Value>,
    ) -> Result<Value, KernelError> {
        let Some(engine) = &self.0.engine else {
            return next.run(input).await;
        };
        let skip = CALLING
            .try_with(|calling| {
                calling
                    .iter()
                    .filter(|(name, _)| name == self.id())
                    .map(|(_, hook)| *hook)
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();
        let site = json!({"session": cx.session, "turn": cx.turn, "item": cx.item, "cwd": cx.cwd,
            "skip": skip, "origin": {"plugin": self.id(), "tier": next.tier()}});
        let mut call = engine.call(&entry.name, input, site).map_err(failed)?;
        let mut next = Some(next);
        loop {
            tokio::select! {
                result = &mut call.result => return result.map_err(failed)?.map_err(failed),
                Some(operation) = call.operations.recv() => {
                    let result = if operation.method == "next" {
                        match next.take() { Some(next) => next.run(operation.params).await.map_err(|error| error.message), None => Err("next may be called only once".into()) }
                    } else { self.operation(cx, operation.hook, &operation.method, operation.params).await };
                    let _ = operation.reply.send(result);
                }
            }
        }
    }
    async fn observe(&self, entry: &catalogue::Entry, cx: &HookContext, input: &Value) {
        if let Err(error) = self
            .chain(
                entry,
                cx,
                input.clone(),
                Next::new(|_| Box::pin(async { Ok(Value::Null) })),
            )
            .await
        {
            let _ = cx
                .host
                .notice(
                    Level::Warn,
                    "INTERCEPTOR_SKIPPED",
                    &format!("{} ({}): {}", self.id(), entry.name, error.message),
                )
                .await;
        }
    }
}
impl Link {
    async fn operation(
        &self,
        cx: &HookContext,
        hook: usize,
        method: &str,
        params: Value,
    ) -> Result<Value, String> {
        let mut calling = CALLING.try_with(Clone::clone).unwrap_or_default();
        calling.push((self.id().into(), hook));
        CALLING
            .scope(calling, async {
                tokio::time::timeout(
                    HOOK_DEADLINE,
                    bingo_plugin_host::ops::call(
                        &cx.host,
                        self.id(),
                        &self.0.store,
                        method,
                        params,
                    ),
                )
                .await
                .map_err(|_| format!("{method}: host operation deadline exceeded"))?
            })
            .await
    }
}
fn failed(error: impl std::fmt::Display) -> KernelError {
    KernelError::new(ErrorCode::Internal, error.to_string())
}
