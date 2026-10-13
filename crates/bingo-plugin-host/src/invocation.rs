//! Calling hooks follow an operation, never unrelated work on the same runtime.
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::future::Future;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct CallingHook {
    pub plugin: String,
    pub hook: usize,
}

tokio::task_local! { static CALLING: Vec<CallingHook>; }

pub fn current() -> Vec<CallingHook> {
    CALLING.try_with(Clone::clone).unwrap_or_default()
}

pub async fn scope<T>(calling: Vec<CallingHook>, future: impl Future<Output = T>) -> T {
    CALLING.scope(calling, future).await
}

pub async fn from<T>(plugin: &str, hook: usize, future: impl Future<Output = T>) -> T {
    let mut calling = current();
    calling.push(CallingHook {
        plugin: plugin.into(),
        hook,
    });
    scope(calling, future).await
}
