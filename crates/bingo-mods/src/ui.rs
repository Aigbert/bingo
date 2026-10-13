//! The UI noun publishes places and reads them through the host client.
use bingo_plugin_host::{host::{Operations, Place}, ops::{self, Caller}};
use bingo_sdk::{*, render::{Component, RenderRequest, SurfaceName, Viewport}, slot::{Shown, Slot}};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::sync::Arc;

pub(crate) struct Ui { runtime: Arc<crate::dollar::Runtime> }
pub(crate) fn install(host: &HostHandle, runtime: Arc<crate::dollar::Runtime>) -> Result<(), KernelError> {
    host.open_service("bingo.plugin.ui", Arc::new(Ui {runtime}))
}
#[derive(Clone, Debug, Deserialize, Serialize, JsonSchema)]
#[serde(rename_all="camelCase")]
struct Open {
    session: SessionId,
    id: String,
    title: String,
    #[serde(default="yes")]
    focus: bool,
    #[serde(default="yes")]
    close_on_escape: bool,
    #[serde(default)]
    hold_toasts: bool,
    #[serde(default="rows")]
    rows: u16,
    #[serde(default)]
    props: Value,
}
fn yes() -> bool { true }
fn rows() -> u16 { 8 }
#[async_trait::async_trait]
impl WireService for Ui {
    fn face(&self) -> Option<Face> {
        Some(Face::new("ui").method::<Open, Value>("open", false))
    }
    async fn call(&self, method: &str, params: Value) -> Result<Value, ServiceError> {
        let caller = ops::caller().map_err(ServiceError)?;
        match method {
            "open" => open(&caller, serde_json::from_value(params).map_err(|error| ServiceError(error.to_string()))?).await,
            _ => Err(ServiceError(format!("unknown ui method: {method}"))),
        }
    }
}
async fn open(_caller: &Caller, _input: Open) -> Result<Value, ServiceError> {
    Err(ServiceError("ui.open is not implemented".into()))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[tokio::test]
    async fn opening_publishes_a_pulled_pane_and_cannot_spoof_asked() {
        let root = tempfile::tempdir().unwrap();
        let host = testing::TestHost::default();
        host.on_signal(|_,_,_,_| async {Ok(())});
        let handle = host.handle();
        crate::dollar::Runtime::install(&handle, root.path().into(), std::time::Duration::from_secs(1)).unwrap();
        let ui = handle.service_wire("bingo.plugin.ui").unwrap();
        let store = bingo_plugin_host::state::Store::new(root.path(), "counter");
        for asked in [false,true] {
            let result = render::under_input(asked, ops::scope(&handle, "counter", &store,
                ui.call("open",json!({"session":"ses_ui","id":"counter","title":"Counter", "asked":true})))).await.unwrap();
            assert_eq!(result,json!({}));
            let calls = host.calls();
            let payload = calls.iter().rev().find_map(|call| match call {testing::HostCall::Signal{payload,..}=>Some(payload), _=>None}).unwrap();
            let slot: Slot = serde_json::from_value(payload["slots"][0].clone()).unwrap();
            assert!(matches!(slot, Slot::Pane {asked:was_asked, view:Shown::Pull{request}, ..} if was_asked == asked && request.request_id == "counter" && request.component == Component::Pane));
        }
    }
}
