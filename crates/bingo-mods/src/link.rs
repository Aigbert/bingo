//! A mod is one interceptor; its registered hooks compose within that link.
use crate::{chain::Chain, module::Module};
use async_trait::async_trait;
use bingo_sdk::*;
use serde_json::Value;
use std::sync::{Arc, Mutex};

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
                .flat_map(|engine| engine.hooks())
                .filter_map(|hook| catalogue::Pattern::parse(&hook.pattern).ok())
                .collect(),
            prefilter: Value::Null,
        }
    }
    fn owns_deadline(&self) -> bool {
        true
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
        let mut hooks = Vec::new();
        for (index, hook) in engine.hooks().iter().enumerate() {
            if !catalogue::Pattern::parse(&hook.pattern)
                .is_ok_and(|pattern| pattern.matches(&entry.name))
            {
                continue;
            }
            let skipped = CALLING
                .try_with(|calling| {
                    calling
                        .iter()
                        .any(|(name, hook)| name == self.id() && *hook == index)
                })
                .unwrap_or(false);
            if skipped {
                let _ = cx
                    .host
                    .notice(
                        Level::Warn,
                        "INTERCEPTOR_SKIPPED",
                        &format!(
                            "{} ({}): re-entry: calling hook skipped",
                            self.id(),
                            entry.name
                        ),
                    )
                    .await;
            } else {
                hooks.push(index);
            }
        }
        let chain = Chain {
            module: self.0.clone(),
            engine: engine.clone(),
            entry: entry.clone(),
            cx: cx.clone(),
            tier: next.tier(),
            hooks,
            kernel: Arc::new(Mutex::new(Some(next))),
        };
        chain.at(0, input).await
    }
    async fn observe(&self, entry: &catalogue::Entry, cx: &HookContext, input: &Value) {
        if let Err(error) = self
            .chain(
                entry,
                cx,
                input.clone(),
                Next::at(
                    self.0
                        .admission
                        .decision()
                        .ok()
                        .flatten()
                        .map_or(Tier::Tighten, |grant| grant.tier),
                    |_| Box::pin(async { Ok(Value::Null) }),
                ),
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
pub(crate) async fn operation(
    module: &Module,
    cx: &HookContext,
    hook: usize,
    method: &str,
    params: Value,
) -> Result<Value, String> {
    let mut calling = CALLING.try_with(Clone::clone).unwrap_or_default();
    calling.push((module.found.name.clone(), hook));
    CALLING
        .scope(calling, async {
            tokio::time::timeout(
                HOOK_DEADLINE,
                bingo_plugin_host::ops::call(
                    &cx.host,
                    &module.found.name,
                    &module.store,
                    method,
                    params,
                ),
            )
            .await
            .map_err(|_| format!("{method}: host operation deadline exceeded"))?
        })
        .await
}
pub(crate) fn failed(error: impl std::fmt::Display) -> KernelError {
    KernelError::new(ErrorCode::Internal, error.to_string())
}
