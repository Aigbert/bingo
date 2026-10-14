//! The host-side handle never executes JavaScript.
use crate::meter::Meter;
use bingo_sdk::CancellationToken;
use serde_json::Value;
use std::{
    path::Path,
    sync::{Arc, Mutex},
    time::Duration,
};
use tokio::sync::{mpsc, oneshot};

#[derive(Clone, Debug)]
pub struct Limits {
    pub memory_bytes: usize,
    pub stack_bytes: usize,
    pub hook_time: Duration,
    pub every_time: Duration,
}
impl Default for Limits {
    fn default() -> Self {
        Self {
            memory_bytes: 64 * 1024 * 1024,
            stack_bytes: 512 * 1024,
            hook_time: Duration::from_secs(10),
            every_time: Duration::from_secs(1),
        }
    }
}
#[derive(Debug, thiserror::Error)]
#[error("{kind}: {message}")]
pub struct Fault {
    pub kind: &'static str,
    pub message: String,
}
impl Fault {
    pub(crate) fn load(message: impl ToString) -> Self {
        Self {
            kind: "load",
            message: message.to_string(),
        }
    }
}

#[derive(Clone, Debug, serde::Deserialize)]
pub struct RegisteredHook {
    pub matcher: Value,
    pub pattern: String,
    pub recover: bool,
}

pub struct Operation {
    pub hook: usize,
    pub method: String,
    pub params: Value,
    pub reply: oneshot::Sender<Result<Value, String>>,
}
pub struct Call {
    pub reads: bingo_plugin_host::state::ReadLog,
    pub operations: mpsc::UnboundedReceiver<Operation>,
    pub result: oneshot::Receiver<Result<Value, Fault>>,
    cancel: CancellationToken,
}
impl Drop for Call {
    fn drop(&mut self) {
        self.cancel.cancel();
    }
}
pub(crate) struct Request {
    pub event: String,
    pub input: Value,
    pub site: Value,
    pub operations: mpsc::UnboundedSender<Operation>,
    pub result: oneshot::Sender<Result<Value, Fault>>,
    pub meter: Arc<Meter>,
}

pub struct Engine {
    requests: mpsc::UnboundedSender<Request>,
    stop: CancellationToken,
    thread: Mutex<Option<std::thread::JoinHandle<()>>>,
    hooks: Vec<RegisteredHook>,
    limits: Limits,
}
impl Engine {
    pub async fn start(
        root: &Path,
        module: &str,
        config: Value,
        limits: Limits,
    ) -> Result<Arc<Self>, Fault> {
        let root = root.to_path_buf();
        let (requests, receiver) = mpsc::unbounded_channel();
        let (ready, started) = oneshot::channel();
        let stop = CancellationToken::new();
        let shutdown = stop.clone();
        let worker_limits = limits.clone();
        let module = module.to_string();
        let thread = std::thread::Builder::new()
            .name("bingo-mod".into())
            .spawn(move || {
                crate::thread::run(
                    root,
                    module,
                    config,
                    worker_limits,
                    shutdown,
                    receiver,
                    ready,
                );
            })
            .map_err(Fault::load)?;
        let mut starting = Starting(Some(stop.clone()));
        let started = started.await.map_err(Fault::load)?;
        starting.0.take();
        match started {
            Ok(hooks) => Ok(Arc::new(Self {
                requests,
                stop,
                thread: Mutex::new(Some(thread)),
                hooks,
                limits,
            })),
            Err(error) => {
                let _ = tokio::task::spawn_blocking(move || thread.join()).await;
                Err(error)
            }
        }
    }
    pub(crate) fn stopped(&self) -> CancellationToken {
        self.stop.clone()
    }
    pub fn hooks(&self) -> &[RegisteredHook] {
        &self.hooks
    }
    pub fn patterns(&self) -> Vec<&str> {
        self.hooks
            .iter()
            .map(|hook| hook.pattern.as_str())
            .collect()
    }
    pub fn call(&self, event: &str, input: Value, site: Value) -> Result<Call, Fault> {
        let (operations, incoming) = mpsc::unbounded_channel();
        let (result, answer) = oneshot::channel();
        let cancel = self.stop.child_token();
        let hook_time = if site.get("recover").and_then(Value::as_bool) == Some(true) {
            Duration::from_secs(1)
        } else {
            self.limits.hook_time
        };
        self.requests
            .send(Request {
                event: event.into(),
                input,
                site,
                operations,
                result,
                meter: Meter::new(hook_time, cancel.clone()),
            })
            .map_err(Fault::load)?;
        Ok(Call {
            reads: Default::default(),
            operations: incoming,
            result: answer,
            cancel,
        })
    }
    pub fn hook_time(&self) -> Duration {
        self.limits.hook_time
    }
    pub async fn close(&self) {
        self.stop.cancel();
        let thread = self.thread.lock().unwrap_or_else(|e| e.into_inner()).take();
        if let Some(thread) = thread {
            let _ = tokio::task::spawn_blocking(move || thread.join()).await;
        }
    }
}
impl Drop for Engine {
    fn drop(&mut self) {
        self.stop.cancel();
    }
}

struct Starting(Option<CancellationToken>);
impl Drop for Starting {
    fn drop(&mut self) {
        if let Some(stop) = self.0.take() {
            stop.cancel();
        }
    }
}
