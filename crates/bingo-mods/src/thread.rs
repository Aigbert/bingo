//! One current-thread executor owns one mod's runtime and all of its jobs.
use crate::{
    engine::{Fault, Limits, Operation, RegisteredHook, Request},
    load::Root,
    meter::Meter,
};
use bingo_sdk::CancellationToken;
use futures::{StreamExt, stream::FuturesUnordered};
use rquickjs::{
    AsyncContext, AsyncRuntime, CaughtError, Function, Module, Promise, prelude::Async,
};
use serde_json::{Value, json};
use std::sync::{Arc, Mutex};
use tokio::sync::{mpsc, oneshot};

type Active = Arc<Mutex<Option<Arc<Meter>>>>;

pub(crate) fn run(
    root: std::path::PathBuf,
    module: String,
    config: Value,
    limits: Limits,
    stop: CancellationToken,
    requests: mpsc::UnboundedReceiver<Request>,
    ready: oneshot::Sender<Result<Vec<RegisteredHook>, Fault>>,
) {
    match tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
    {
        Ok(runtime) => runtime.block_on(serve(root, module, config, limits, stop, requests, ready)),
        Err(error) => {
            let _ = ready.send(Err(Fault::load(error)));
        }
    }
}

async fn serve(
    root: std::path::PathBuf,
    module: String,
    config: Value,
    limits: Limits,
    stop: CancellationToken,
    mut requests: mpsc::UnboundedReceiver<Request>,
    ready: oneshot::Sender<Result<Vec<RegisteredHook>, Fault>>,
) {
    let active = Active::default();
    let loaded = initialize(root, &module, &config, &limits, &stop, &active).await;
    let (runtime, context, patterns) = match loaded {
        Ok(loaded) => loaded,
        Err(error) => {
            let _ = ready.send(Err(error));
            return;
        }
    };
    if ready.send(Ok(patterns)).is_err() {
        return;
    }
    let drive = tokio::spawn(runtime.drive());
    let mut jobs = FuturesUnordered::new();
    loop {
        tokio::select! {
            biased;
            _ = stop.cancelled() => break,
            Some(request) = requests.recv() => jobs.push(execute(context.clone(), active.clone(), request)),
            _ = jobs.next(), if !jobs.is_empty() => (),
            else => break,
        }
    }
    drive.abort();
    let _ = drive.await;
    drop(jobs);
    drop(context);
    runtime.run_gc().await;
}

async fn initialize(
    root: std::path::PathBuf,
    module: &str,
    config: &Value,
    limits: &Limits,
    stop: &CancellationToken,
    active: &Active,
) -> Result<(AsyncRuntime, AsyncContext, Vec<RegisteredHook>), Fault> {
    let root = Root::new(&root, limits.memory_bytes as u64).map_err(Fault::load)?;
    let runtime = AsyncRuntime::new().map_err(Fault::load)?;
    runtime.set_memory_limit(limits.memory_bytes).await;
    runtime.set_max_stack_size(limits.stack_bytes).await;
    runtime.set_loader(root.clone(), root).await;
    let meter = Meter::new(limits.hook_time, stop.child_token());
    meter.resume();
    *active.lock().unwrap_or_else(|e| e.into_inner()) = Some(meter.clone());
    let current = active.clone();
    let shutdown = stop.clone();
    runtime
        .set_interrupt_handler(Some(Box::new(move || {
            shutdown.is_cancelled()
                || current
                    .lock()
                    .unwrap_or_else(|e| e.into_inner())
                    .as_ref()
                    .is_some_and(|m| m.interrupt())
        })))
        .await;
    let context = crate::sandbox::context(&runtime)
        .await
        .map_err(Fault::load)?;
    let bootstrap = format!(
        "import {{ Hooks, dollar, Callbacks, Handlers, patternMatches, matches, frozen, renderMatcher }} from 'bingo';\nimport {{ register }} from {};\nconst hooks = new Hooks();\nconst callbacks = new Callbacks();\nconst handlers = new Handlers();\nawait register(hooks.on, {});\n{}",
        json!(format!("./{module}")),
        config,
        include_str!("thread.js")
    );
    let loaded = context.async_with(async |ctx| {
        let (module, promise) = Module::declare(ctx.clone(), "__runtime__.js", bootstrap)
            .and_then(Module::eval)
            .map_err(|e| Fault::load(CaughtError::from_error(&ctx, e)))?;
        promise
            .into_future::<()>()
            .await
            .map_err(|e| Fault::load(CaughtError::from_error(&ctx, e)))?;
        let run: Function = module.get("run").map_err(Fault::load)?;
        let metadata: Function = module.get("metadata").map_err(Fault::load)?;
        let patterns: String = metadata.call(()).map_err(Fault::load)?;
        ctx.globals()
            .set("__bingo_dispatch", run)
            .map_err(Fault::load)?;
        serde_json::from_str::<Vec<RegisteredHook>>(&patterns).map_err(Fault::load)
    });
    let patterns = tokio::select! {
        result = loaded => result?,
        _ = meter.deadline() => return Err(Fault::load("module registration exceeded its budget or was cancelled")),
    };
    meter.suspend();
    Ok((runtime, context, patterns))
}

async fn execute(context: AsyncContext, active: Active, request: Request) {
    let Request {
        event,
        input,
        site,
        operations,
        result,
        meter,
    } = request;
    let watching = meter.clone();
    let running = context.async_with(async move |ctx| {
        meter.resume();
        *active.lock().unwrap_or_else(|e| e.into_inner()) = Some(meter.clone());
        let clock = meter.clone();
        let current = active.clone();
        let ask = Function::new(
            ctx.clone(),
            Async(move |method: String, params: String, hook: usize| {
                ask(
                    hook,
                    method,
                    params,
                    operations.clone(),
                    clock.clone(),
                    current.clone(),
                )
            }),
        )
        .map_err(Fault::load)?;
        let remaining = meter.clone();
        let remaining = Function::new(ctx.clone(), move || {
            remaining.remaining().as_millis() as f64
        })
        .map_err(Fault::load)?;
        let flag = meter.cancel.clone();
        let cancelled =
            Function::new(ctx.clone(), move || flag.is_cancelled()).map_err(Fault::load)?;
        let function: Function = ctx.globals().get("__bingo_dispatch").map_err(Fault::load)?;
        let promise: Promise = function
            .call((
                event,
                input.to_string(),
                site.to_string(),
                ask,
                cancelled,
                remaining,
            ))
            .map_err(|e| fault(&ctx, e, &meter))?;
        let answer = promise
            .into_future::<String>()
            .await
            .map_err(|e| fault(&ctx, e, &meter))?;
        serde_json::from_str(&answer).map_err(Fault::load)
    });
    let outcome = tokio::select! {
        outcome = running => outcome,
        _ = watching.deadline() => Err(Fault { kind: "timeout", message: "hook exceeded its budget or was cancelled".into() }),
    };
    watching.suspend();
    let _ = result.send(outcome);
}

async fn ask(
    hook: usize,
    method: String,
    params: String,
    operations: mpsc::UnboundedSender<Operation>,
    meter: Arc<Meter>,
    active: Active,
) -> String {
    meter.suspend();
    let (reply, answer) = oneshot::channel();
    let result = match serde_json::from_str(&params) {
        Ok(params) => {
            if operations
                .send(Operation {
                    hook,
                    method,
                    params,
                    reply,
                })
                .is_err()
            {
                Err("host call was dropped".into())
            } else {
                tokio::select! {
                    result = answer => result.unwrap_or_else(|_| Err("host call was dropped".into())),
                    _ = meter.cancel.cancelled() => Err("hook cancelled".into()),
                }
            }
        }
        Err(error) => Err(format!("invalid host call: {error}")),
    };
    meter.resume();
    *active.lock().unwrap_or_else(|e| e.into_inner()) = Some(meter);
    match result {
        Ok(value) => json!({"ok": true, "value": value}).to_string(),
        Err(error) => json!({"ok": false, "error": error}).to_string(),
    }
}

fn fault(ctx: &rquickjs::Ctx<'_>, error: rquickjs::Error, meter: &Meter) -> Fault {
    let message = CaughtError::from_error(ctx, error).to_string();
    let kind = if meter.interrupt() {
        "timeout"
    } else if message.to_lowercase().contains("memory") {
        "memory"
    } else {
        "throw"
    };
    Fault { kind, message }
}
