//! Each registered hook has its own stage, continuation and recovery budget.
use crate::{
    engine::{Engine, Fault},
    link,
    module::Module,
};
use bingo_sdk::*;
use futures::{
    FutureExt,
    future::{BoxFuture, Shared},
};
use serde_json::{Value, json};
use std::sync::{Arc, Mutex};

type Answer = Result<Value, KernelError>;
type Pending<'a> = Shared<BoxFuture<'a, Answer>>;

pub(crate) struct Chain<'a> {
    pub module: Arc<Module>,
    pub engine: Arc<Engine>,
    pub entry: catalogue::Entry,
    pub cx: HookContext,
    pub tier: Tier,
    pub hooks: Vec<usize>,
    pub reads: bingo_plugin_host::state::ReadLog,
    pub generation: String,
    pub kernel: Arc<Mutex<Option<Next<'a, Value, Value>>>>,
}
#[derive(serde::Deserialize)]
struct Returned {
    answered: bool,
    value: Value,
}
impl Chain<'_> {
    pub fn at<'a>(&'a self, position: usize, input: Value) -> BoxFuture<'a, Answer> {
        Box::pin(async move {
            let Some(&hook) = self.hooks.get(position) else {
                let next = self
                    .kernel
                    .lock()
                    .unwrap_or_else(|error| error.into_inner())
                    .take();
                return match next {
                    Some(next) => next.run(input).await,
                    None => Err(link::failed("next already consumed")),
                };
            };
            let mut pending = None;
            let result = self.stage(position, hook, &input, &mut pending, None).await;
            match result {
                Ok(returned) if returned.answered => Ok(returned.value),
                Ok(_) => self.below(position, input, pending).await,
                Err(error) => self.recover(position, hook, input, pending, error).await,
            }
        })
    }
    async fn below<'a>(
        &'a self,
        position: usize,
        input: Value,
        pending: Option<Pending<'a>>,
    ) -> Answer {
        match pending {
            Some(pending) => pending.await,
            None => self.at(position + 1, input).await,
        }
    }
    async fn recover<'a>(
        &'a self,
        position: usize,
        hook: usize,
        input: Value,
        mut pending: Option<Pending<'a>>,
        error: Fault,
    ) -> Answer {
        let result = if self.engine.hooks()[hook].recover {
            self.stage(position, hook, &input, &mut pending, Some(&error))
                .await
                .ok()
        } else {
            None
        };
        let _ = self
            .cx
            .host
            .notice(
                Level::Warn,
                "INTERCEPTOR_SKIPPED",
                &format!("{} ({}): {error}", self.module.found.name, self.entry.name),
            )
            .await;
        match result {
            Some(returned) if returned.answered => Ok(returned.value),
            _ => self.below(position, input, pending).await,
        }
    }
    async fn stage<'a>(
        &'a self,
        position: usize,
        hook: usize,
        input: &Value,
        pending: &mut Option<Pending<'a>>,
        error: Option<&Fault>,
    ) -> Result<Returned, Fault> {
        let ms = if error.is_some() {
            1000
        } else {
            self.engine.hook_time().as_millis()
        };
        let site = json!({"session":self.cx.session, "turn":self.cx.turn, "item":self.cx.item, "cwd":self.cx.cwd,
            "plugin":self.module.identity(),
            "generation":self.generation, "hook":hook, "recover":error.is_some(), "called":pending.is_some(), "budget":{"ms":ms,"remainingMs":ms},
            "error":error.map(|error| json!({"kind":error.kind,"message":error.message})),
            "origin":{"plugin":self.module.found.name,"tier":self.tier}});
        let mut call = self.engine.call(&self.entry.name, input.clone(), site)?;
        loop {
            tokio::select! {
                biased;
                Some(operation) = call.operations.recv() => {
                    let result = if operation.method == "next" {
                        self.continue_with(position, input, operation.params, pending).await
                    } else {
                        bingo_plugin_host::state::rendering(self.entry == *catalogue::UI_RENDER,
                            self.reads.scope(link::operation(&self.module, &self.cx, hook, &operation.method, operation.params))).await
                    };
                    let _ = operation.reply.send(result);
                }
                result = &mut call.result => {
                    let value = result.map_err(Fault::load)??;
                    let returned: Returned = serde_json::from_value(value).map_err(Fault::load)?;
                    if returned.answered && self.entry.kind != catalogue::Kind::Observation && !(self.entry.valid_result)(&returned.value) {
                        return Err(Fault { kind:"shape", message:format!("{}: invalid result shape", self.entry.name) });
                    }
                    return Ok(returned);
                }
            }
        }
    }
    async fn continue_with<'a>(
        &'a self,
        position: usize,
        original: &Value,
        changed: Value,
        pending: &mut Option<Pending<'a>>,
    ) -> Result<Value, String> {
        if pending.is_some() {
            return Err("next may be called only once".into());
        }
        if !(self.entry.valid_input)(&changed)
            || catalogue::pinned_changed(original, &changed, self.entry.pinned).is_some()
        {
            return Err(format!(
                "shape: {}: invalid input or pinned field",
                self.entry.name
            ));
        }
        let future = self.at(position + 1, changed).shared();
        *pending = Some(future.clone());
        future.await.map_err(|error| error.message)
    }
}
