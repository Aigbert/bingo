//! Context construction for one isolated mod.
use rquickjs::{AsyncContext, AsyncRuntime};

pub async fn context(runtime: &AsyncRuntime) -> rquickjs::Result<AsyncContext> {
    AsyncContext::full(runtime).await
}
