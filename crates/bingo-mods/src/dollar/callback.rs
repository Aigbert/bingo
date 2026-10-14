//! A callback runs on its module thread with the ordinary hook meter.
use crate::{manager::Manager, module::Module};
use bingo_plugin_host::callback::Executor;
use bingo_sdk::*;
use serde_json::{Value, json};
use std::sync::{Arc, Weak};

pub(crate) struct Native {
    pub module: Arc<Module>,
    pub manager: Weak<Manager>,
}
#[async_trait::async_trait]
impl Executor for Native {
    fn stopped(&self) -> CancellationToken {
        self.module
            .engine
            .as_ref()
            .map(|engine| engine.stopped())
            .unwrap_or_else(|| {
                let stop = CancellationToken::new();
                stop.cancel();
                stop
            })
    }
    async fn prepare(&self) -> Result<(), String> {
        self.manager
            .upgrade()
            .ok_or("module manager is unavailable")?
            .prepare()
            .await;
        Ok(())
    }
    async fn call(&self, id: &str, input: Value, cx: &HookContext) -> Result<Value, String> {
        if !self.module.admission.allowed() {
            return Err("module is not admitted".into());
        }
        let engine = self.module.engine.as_ref().ok_or("module is unavailable")?;
        let site = json!({"callback":id,"session":cx.session,"turn":cx.turn,"cwd":cx.cwd,
            "plugin":self.module.identity()});
        let mut call = engine
            .call("callback", input, site)
            .map_err(|error| error.to_string())?;
        loop {
            tokio::select! {
                biased;
                Some(operation) = call.operations.recv() => {
                    let result = call.reads.scope(crate::link::operation(&self.module, cx, usize::MAX,
                        &operation.method, operation.params)).await;
                    let _ = operation.reply.send(result);
                }
                answer = &mut call.result => return answer.map_err(|error| error.to_string())?
                    .map_err(|error| error.to_string()),
            }
        }
    }
}
