//! The process transport for module callbacks.
use bingo_plugin_host::callback::Executor;
use bingo_sdk::{CancellationToken, HookContext};
use serde_json::Value;
use std::sync::Arc;

pub(crate) struct Remote {
    pub connection: Arc<crate::connection::Connection>,
    pub bridge: std::sync::Weak<crate::Bridge>,
}
#[async_trait::async_trait]
impl Executor for Remote {
    fn stopped(&self) -> CancellationToken {
        self.connection.stopped()
    }
    async fn prepare(&self) -> Result<(), String> {
        self.bridge
            .upgrade()
            .ok_or("callback bridge is unavailable")?
            .refresh()
            .await;
        Ok(())
    }
    async fn call(&self, id: &str, input: Value, cx: &HookContext) -> Result<Value, String> {
        let chain = self.connection.next_call();
        let _held = self.connection.hold_chain(&chain);
        let params = crate::wire::CallbackParams {
            chain,
            callback: id.into(),
            input,
            site: cx.into(),
        };
        tokio::time::timeout(
            bingo_sdk::HOOK_DEADLINE,
            self.connection.request(
                "callback/run",
                serde_json::to_value(params).map_err(|error| error.to_string())?,
            ),
        )
        .await
        .map_err(|_| "callback exceeded its hook budget".to_string())?
        .map_err(|error| error.message)
    }
}

pub(crate) async fn restart(
    connection: &crate::connection::Connection,
    host: &bingo_sdk::HostHandle,
) -> Result<(), String> {
    if !connection.permits("callback/start")
        || host.service(&bingo_plugin_host::callback::KEY).is_none()
    {
        return Ok(());
    }
    for session in host
        .sessions(bingo_sdk::SessionFilter::default())
        .await
        .map_err(|error| error.to_string())?
    {
        let site = crate::wire::HookSite {
            session: session.id,
            turn: None,
            cwd: session.cwd.into(),
            model: session.choice.model,
        };
        tokio::time::timeout(
            bingo_sdk::HOOK_DEADLINE,
            connection.request(
                "callback/start",
                serde_json::to_value(site).map_err(|error| error.to_string())?,
            ),
        )
        .await
        .map_err(|_| "callback restart exceeded its hook budget".to_string())?
        .map_err(|error| error.message)?;
    }
    Ok(())
}
