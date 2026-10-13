//! One spawned plugin process, and the line loop that talks to it.
//!
//! The reader owns the child's stdout and nothing else: it hands each reply to
//! whoever is waiting for that id, each `tool/progress` to whoever is running
//! that call, and each `provider/delta` to whoever is reading that stream. When
//! the pipe closes it fails every waiting request at once, so a call whose
//! process died returns instead of hanging — nothing else would wake it but a
//! person stopping the turn.
//!
//! A stream's queue is bounded, and the reader waits on it: a process that
//! writes faster than a turn reads blocks on its own pipe rather than growing a
//! queue in this one.
//!
//! One kind of line comes the other way round: a request. Exactly one method
//! is served — `service/call` (ADR-0031 §4) — on a task of its own, so a slow
//! service never stops this reader; every other method a process asks for
//! keeps the refusal it has always had, because what more a process may ask
//! the host is a decision to be taken, not a door to be widened here.

use std::collections::HashMap;
use std::path::Path;
use std::process::Stdio;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, MutexGuard};

use bingo_jsonrpc::{Handler, Peer, PeerError};
use bingo_process::Tree;
use bingo_sdk::{ModelEvent, ToolHost};
use futures::FutureExt;
use serde_json::Value;
use tokio::process::{ChildStdin, ChildStdout, Command};
use tokio::sync::mpsc::{Sender, UnboundedSender};

use crate::codec::{Message, Notification, Request, Response, RpcError, TRANSPORT_ERROR};
use crate::manifest::Entry;
use crate::service::ServiceCalls;
use crate::wire::{ProviderDeltaParams, ToolProgressParams, name};

/// What a request came back with.
pub type Reply = Result<Value, RpcError>;

/// `<data_dir>/logs/plugin-<name>.log`.
pub fn log_path(data_dir: &Path, plugin: &str) -> std::path::PathBuf {
    data_dir.join("logs").join(format!("plugin-{plugin}.log"))
}

/// Who answers the one request a process may send, and how the answer gets
/// back to it. Absent where nobody is serving — a connection a test drives,
/// a host that keeps no services — and then that request is refused like any
/// other (ADR-0031 §4).
struct Serving {
    calls: Arc<dyn ServiceCalls>,
}

impl std::fmt::Debug for Serving {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Serving")
    }
}

impl Serving {
    /// On a task of its own: a service that takes a second must not stop the
    /// reader, which is also carrying this process's replies and deltas.
    async fn answer(&self, request: Request, writer: bingo_jsonrpc::Sender) {
        let calls = &self.calls;
        let answered = match calls.call(request.params).await {
            Ok(result) => Response::ok(request.id, result),
            Err(error) => Response::failed(Some(request.id), error),
        };
        if let Err(why) = writer.send(Message::Response(answered)) {
            tracing::debug!(%why, "an answer to a plugin went nowhere");
        }
    }
}

/// Whether the host answers a request a process sent at all. Exactly one
/// method, by name (ADR-0031 §4).
fn served(method: &str) -> bool {
    method == name::SERVICE_CALL
}

/// One tool call this process is running: where its progress line goes, and
/// the host it is a call of.
///
/// Both are the same fact — that the call is live — so they are one entry.
/// The host is what a question on this call is put through (ADR-0033 §3): the
/// call's own asking machinery, which is every in-process tool's too.
struct Running {
    tail: UnboundedSender<String>,
    call: Arc<dyn ToolHost>,
}

impl std::fmt::Debug for Running {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Running")
    }
}

/// Where a reply and a progress line go, whether the process is still there
/// to send either, and who answers the one thing it may ask.
#[derive(Debug, Default)]
struct Router {
    #[cfg(test)]
    alive: AtomicBool,
    /// Set by whoever first reports the death, so it is reported once.
    announced: AtomicBool,
    running: Mutex<HashMap<String, Running>>,
    streaming: Mutex<HashMap<String, Sender<ModelEvent>>>,
    serving: Option<Serving>,
}

impl Router {
    fn running(&self) -> MutexGuard<'_, HashMap<String, Running>> {
        self.running.lock().unwrap_or_else(|held| held.into_inner())
    }

    fn streaming(&self) -> MutexGuard<'_, HashMap<String, Sender<ModelEvent>>> {
        self.streaming
            .lock()
            .unwrap_or_else(|held| held.into_inner())
    }

    #[cfg(test)]
    async fn line(&self, plugin: &str, line: &str) {
        match serde_json::from_str::<Message>(line) {
            Ok(Message::Notification(note)) => self.notification(plugin, note).await,
            Ok(Message::Request(request)) => self.asked(plugin, request, None).await,
            _ => {}
        }
    }

    async fn asked(&self, plugin: &str, request: Request, sender: Option<bingo_jsonrpc::Sender>) {
        match (
            self.serving.as_ref().filter(|_| served(&request.method)),
            sender,
        ) {
            (Some(serving), Some(sender)) => serving.answer(request, sender).await,
            _ => {
                tracing::warn!(plugin, method = %request.method, "a plugin asked, which it may not")
            }
        }
    }

    async fn notification(&self, plugin: &str, notification: Notification) {
        match notification.method.as_str() {
            name::TOOL_PROGRESS => match serde_json::from_value(notification.params) {
                Ok(params) => self.progress(params),
                Err(error) => tracing::warn!(plugin, %error, "a progress line that is not one"),
            },
            name::PROVIDER_DELTA => match serde_json::from_value(notification.params) {
                Ok(params) => self.delta(params).await,
                Err(error) => tracing::warn!(plugin, %error, "a delta that is not an event"),
            },
            other => tracing::debug!(plugin, method = other, "an unknown notification"),
        }
    }

    fn progress(&self, params: ToolProgressParams) {
        if let Some(running) = self.running().get(&params.call_id) {
            // A call that has already answered still has its sink until the
            // guard drops; a send nobody reads is not a problem.
            let _ = running.tail.send(params.tail);
        }
    }

    /// One event, to the one stream it names. The queue is bounded, so this
    /// waits when a process outruns the turn reading it — the pipe is where the
    /// backlog belongs. A stream nobody is reading any more has no sink, and
    /// its events go nowhere.
    async fn delta(&self, params: ProviderDeltaParams) {
        let sink = self.streaming().get(&params.call).cloned();
        if let Some(sink) = sink {
            let _ = sink.send(params.event).await;
        }
    }

    /// The pipe closed: nothing more will ever arrive, so everything waiting
    /// fails now rather than never.
    fn closed(&self) {
        #[cfg(test)]
        self.alive.store(false, Ordering::Release);
        self.running().clear();
        self.streaming().clear();
    }
}

/// One live plugin process.
pub struct Connection {
    plugin: String,
    child: tokio::sync::Mutex<Tree>,
    peer: Peer,
    router: Arc<Router>,
}

impl std::fmt::Debug for Connection {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Connection")
            .field("plugin", &self.plugin)
            .field("alive", &self.is_alive())
            .finish_non_exhaustive()
    }
}

impl Connection {
    /// Spawn the entry, or say why not. The child's working directory is the
    /// plugin's own, so a script that reads a file beside itself finds it; the
    /// directory a call is about travels in the call, never in the process.
    pub fn spawn(
        plugin: &str,
        entry: &Entry,
        root: &Path,
        data_dir: &Path,
        serving: Option<Arc<dyn ServiceCalls>>,
    ) -> Result<Self, String> {
        let mut command = Command::new(&entry.command);
        command
            .args(&entry.args)
            .envs(&entry.env)
            .current_dir(root)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(stderr_sink(data_dir, plugin));
        let mut child =
            bingo_process::tree(command).map_err(|e| format!("spawning {}: {e}", entry.command))?;
        let (Some(stdin), Some(stdout)) = (child.take_stdin(), child.take_stdout()) else {
            return Err("the child was spawned without pipes".to_string());
        };
        Ok(Self::over(plugin, child, stdin, stdout, serving))
    }

    fn over(
        plugin: &str,
        child: Tree,
        stdin: ChildStdin,
        stdout: ChildStdout,
        serving: Option<Arc<dyn ServiceCalls>>,
    ) -> Self {
        let router = Arc::new(Router {
            #[cfg(test)]
            alive: AtomicBool::new(true),
            serving: serving.map(|calls| Serving { calls }),
            ..Router::default()
        });
        let handler = Arc::new(Inbound {
            plugin: plugin.to_string(),
            router: router.clone(),
        });
        let peer = Peer::spawn(stdout, stdin, handler);
        Self {
            plugin: plugin.to_string(),
            child: tokio::sync::Mutex::new(child),
            peer,
            router,
        }
    }

    /// Whether the process is still on the other end of the pipe.
    pub fn is_alive(&self) -> bool {
        self.peer.is_alive()
    }

    /// True for the first caller to find this process gone, so one death is
    /// one notice however many readers noticed it.
    pub fn claim_death(&self) -> bool {
        !self.is_alive() && !self.router.announced.swap(true, Ordering::AcqRel)
    }

    /// Ask, and wait for the answer or for the process to end.
    pub async fn request(&self, method: &str, params: Value) -> Reply {
        self.peer
            .request(method, params)
            .await
            .map_err(|error| match error {
                PeerError::Remote(error) => error,
                PeerError::Closed => RpcError::new(TRANSPORT_ERROR, "the plugin process ended"),
                error => RpcError::new(TRANSPORT_ERROR, error.to_string()),
            })
    }

    pub async fn notify(&self, method: &str, params: Value) {
        if let Err(why) = self.peer.notify(method, params) {
            tracing::debug!(plugin = %self.plugin, method, %why, "a notification went nowhere");
        }
    }

    /// File this call as running until the guard drops: its `tool/progress`
    /// lines go to `sink`, and a question it asks goes to `call`.
    pub fn watch(
        self: &Arc<Self>,
        call_id: &str,
        sink: UnboundedSender<String>,
        call: Arc<dyn ToolHost>,
    ) -> Watch {
        self.router
            .running()
            .insert(call_id.to_string(), Running { tail: sink, call });
        Watch {
            router: Arc::clone(&self.router),
            call_id: call_id.to_string(),
        }
    }

    /// The host to put a question through for a call this process is running,
    /// and nothing for one that has ended or was never this process's
    /// (ADR-0033 §3). The map is this connection's, so another connection's
    /// call is simply not in it.
    pub fn asking(&self, call_id: &str) -> Option<Arc<dyn ToolHost>> {
        self.router
            .running()
            .get(call_id)
            .map(|running| Arc::clone(&running.call))
    }

    /// Route this stream's `provider/delta` events to `sink` until the guard
    /// drops. Two streams on one pipe are two routes, told apart by `call`.
    pub fn watch_stream(self: &Arc<Self>, call: &str, sink: Sender<ModelEvent>) -> StreamWatch {
        self.router.streaming().insert(call.to_string(), sink);
        StreamWatch {
            router: Arc::clone(&self.router),
            call: call.to_string(),
        }
    }

    /// A name for one stream on this pipe, unique while the process lives. The
    /// counter is the request ids', so one process mints one series.
    pub fn next_call(&self) -> String {
        format!("call-{}", self.peer.next_id())
    }

    /// End the process. The host is closing, or the plugin refused the
    /// handshake and will never be spoken to again.
    pub async fn stop(&self) {
        self.peer.close();
        if let Err(error) = self.child.lock().await.start_kill() {
            tracing::debug!(plugin = %self.plugin, %error, "the plugin process was already gone");
        }
    }
}

/// Removes a call's progress route when the call is over.
pub struct Watch {
    router: Arc<Router>,
    call_id: String,
}

impl Drop for Watch {
    fn drop(&mut self) {
        self.router.running().remove(&self.call_id);
    }
}

/// Removes a stream's delta route when nobody is reading it any more.
pub struct StreamWatch {
    router: Arc<Router>,
    call: String,
}

impl Drop for StreamWatch {
    fn drop(&mut self) {
        self.router.streaming().remove(&self.call);
    }
}

struct Inbound {
    plugin: String,
    router: Arc<Router>,
}

impl Handler for Inbound {
    fn handle(
        &self,
        message: Message,
        sender: bingo_jsonrpc::Sender,
    ) -> futures::future::BoxFuture<'_, ()> {
        async move {
            match message {
                Message::Notification(note) => self.router.notification(&self.plugin, note).await,
                Message::Request(request) => {
                    self.router.asked(&self.plugin, request, Some(sender)).await
                }
                Message::Response(_) => {
                    tracing::debug!(plugin = %self.plugin, "a reply nobody was waiting for")
                }
            }
        }
        .boxed()
    }
    fn closed(&self, _: &PeerError) {
        self.router.closed();
    }
}

/// Where a child's stderr goes. Left alone it inherits the terminal and paints
/// over a full-screen TUI, which never redraws its scrollback; a log file that
/// will not open sends it to the void rather than to the screen.
fn stderr_sink(data_dir: &Path, plugin: &str) -> Stdio {
    match open_log(data_dir, plugin) {
        Ok(file) => Stdio::from(file),
        Err(error) => {
            tracing::warn!(
                plugin,
                %error,
                "no log for this plugin's stderr; discarding it"
            );
            Stdio::null()
        }
    }
}

fn open_log(data_dir: &Path, plugin: &str) -> std::io::Result<std::fs::File> {
    let path = log_path(data_dir, plugin);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::File::create(path)
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::*;
    use serde_json::json;

    fn entry(command: &str, args: &[&str]) -> Entry {
        Entry {
            command: command.to_string(),
            args: args.iter().map(|arg| (*arg).to_string()).collect(),
            env: Default::default(),
        }
    }

    #[tokio::test]
    async fn dropping_a_connection_closes_a_grandchilds_stdout() {
        let dir = tempfile::tempdir().expect("tempdir");
        let shell = bingo_process::shell::resolve().expect("shell");
        let command = entry(
            shell.path().to_str().expect("shell path"),
            &[
                "-c",
                "sleep 60 & read line; echo '{\"jsonrpc\":\"2.0\",\"id\":1,\"result\":{}}'; wait",
            ],
        );
        let connection =
            Connection::spawn("tree", &command, dir.path(), dir.path(), None).expect("spawn");
        tokio::time::timeout(
            std::time::Duration::from_secs(10),
            connection.request("ready", json!({})),
        )
        .await
        .expect("ready deadline")
        .expect("ready");
        let router = connection.router.clone();
        drop(connection.child);
        tokio::time::timeout(std::time::Duration::from_secs(3), async {
            while router.alive.load(Ordering::Acquire) {
                tokio::task::yield_now().await;
            }
        })
        .await
        .expect("the grandchild must close stdout when its connection drops");
    }

    #[test]
    fn a_plugin_logs_its_stderr_under_the_data_directory() {
        assert_eq!(
            log_path(Path::new("/home/u/.bingo/data"), "wordcount"),
            PathBuf::from("/home/u/.bingo/data/logs/plugin-wordcount.log")
        );
    }

    #[test]
    fn a_command_that_does_not_exist_says_so_rather_than_pretending() {
        let dir = tempfile::tempdir().expect("a temporary directory");
        let error = Connection::spawn(
            "missing",
            &entry("bingo-no-such-plugin", &[]),
            dir.path(),
            dir.path(),
            None,
        )
        .expect_err("there is no such command");
        assert!(
            error.starts_with("spawning bingo-no-such-plugin"),
            "{error}"
        );
    }

    #[test]
    fn a_log_the_process_cannot_open_sends_stderr_to_the_void_not_the_screen() {
        // A data directory that is a file has no `logs` directory to create.
        let file = tempfile::NamedTempFile::new().expect("a temporary file");
        drop(stderr_sink(file.path(), "wordcount"));
    }

    /// `cat` echoes nothing back, so the request is still waiting when the
    /// process ends: the reader's `closed` is what wakes it.
    #[tokio::test]
    async fn a_request_whose_process_ends_fails_instead_of_hanging() {
        let dir = tempfile::tempdir().expect("a temporary directory");
        let connection =
            Connection::spawn("quiet", &entry("true", &[]), dir.path(), dir.path(), None)
                .expect("`true` exists on every unix");
        let error = tokio::time::timeout(
            crate::deadline::HANDSHAKE,
            connection.request("initialize", json!({})),
        )
        .await
        .expect("a closed pipe answers within the handshake bound")
        .expect_err("a process that says nothing answers nothing");
        assert_eq!(error.code, TRANSPORT_ERROR);
        assert!(!connection.is_alive());
        assert!(connection.claim_death(), "the first to notice reports it");
        assert!(!connection.claim_death(), "and only the first");
    }

    #[tokio::test]
    async fn a_line_that_is_not_a_message_is_ignored_not_fatal() {
        let router = Router {
            alive: AtomicBool::new(true),
            ..Router::default()
        };
        router.line("noisy", "this is not json").await;
        router
            .line("noisy", r#"{"jsonrpc":"2.0","id":1,"result":{}}"#)
            .await;
        assert!(router.alive.load(Ordering::Acquire));
    }

    #[tokio::test]
    async fn a_progress_line_reaches_the_call_it_names_and_no_other() {
        let router = Router {
            alive: AtomicBool::new(true),
            ..Router::default()
        };
        let (sender, mut tail) = tokio::sync::mpsc::unbounded_channel();
        router.running().insert(
            "call_1".to_string(),
            Running {
                tail: sender,
                call: crate::testing::Silent::cancelling(),
            },
        );
        router.line(
            "noisy",
            r#"{"jsonrpc":"2.0","method":"tool/progress","params":{"callId":"call_1","tail":"half way"}}"#,
        ).await;
        router.line(
            "noisy",
            r#"{"jsonrpc":"2.0","method":"tool/progress","params":{"callId":"call_2","tail":"nobody"}}"#,
        ).await;
        assert_eq!(tail.recv().await.as_deref(), Some("half way"));
        assert!(
            tail.try_recv().is_err(),
            "the other call's line went nowhere"
        );
    }

    fn delta(call: &str, text: &str) -> String {
        format!(
            r#"{{"jsonrpc":"2.0","method":"provider/delta","params":{{"call":"{call}","event":{{"type":"textDelta","id":"b1","delta":"{text}"}}}}}}"#
        )
    }

    /// The defect the `call` key exists to make impossible: two streams on one
    /// pipe, and neither sees the other's events.
    #[tokio::test]
    async fn a_delta_reaches_the_stream_it_names_and_no_other() {
        let router = Router {
            alive: AtomicBool::new(true),
            ..Router::default()
        };
        let (one, mut first) = tokio::sync::mpsc::channel(4);
        let (two, mut second) = tokio::sync::mpsc::channel(4);
        router.streaming().insert("call-1".to_string(), one);
        router.streaming().insert("call-2".to_string(), two);
        router.line("noisy", &delta("call-1", "mine")).await;
        router.line("noisy", &delta("call-2", "yours")).await;
        router.line("noisy", &delta("call-3", "nobody's")).await;
        assert_eq!(
            first.recv().await,
            Some(ModelEvent::TextDelta {
                id: "b1".into(),
                delta: "mine".into()
            })
        );
        assert_eq!(
            second.recv().await,
            Some(ModelEvent::TextDelta {
                id: "b1".into(),
                delta: "yours".into()
            })
        );
        assert!(first.try_recv().is_err() && second.try_recv().is_err());
    }

    /// The pipe closed: a stream waiting on it is woken by the sink going,
    /// exactly as a waiting request is woken by its error.
    #[tokio::test]
    async fn a_process_that_ends_ends_every_stream_it_was_feeding() {
        let router = Router {
            alive: AtomicBool::new(true),
            ..Router::default()
        };
        let (sender, mut events) = tokio::sync::mpsc::channel(4);
        router.streaming().insert("call-1".to_string(), sender);
        router.closed();
        assert_eq!(events.recv().await, None);
    }

    #[tokio::test]
    async fn one_process_mints_one_series_of_stream_names() {
        let dir = tempfile::tempdir().expect("a temporary directory");
        let connection =
            Connection::spawn("quiet", &entry("cat", &[]), dir.path(), dir.path(), None)
                .expect("`cat` exists on every unix");
        assert_ne!(connection.next_call(), connection.next_call());
    }

    /// The whole of the reverse lane's width (ADR-0031 §4): one method. What
    /// else a process may ask the host for is a decision nobody has taken.
    #[test]
    fn exactly_one_method_travels_from_a_process_to_the_host() {
        assert!(served(name::SERVICE_CALL));
        for other in [
            name::INITIALIZE,
            name::TOOL_CALL,
            name::COMMAND_RUN,
            name::CONTEXT_CONTRIBUTE,
            name::COMPACTOR_COMPACT,
            name::PROVIDER_STREAM,
        ] {
            assert!(
                !served(other),
                "{other} is the host's to ask, not to answer"
            );
        }
    }

    /// A request nobody serves is refused where it arrives, and the reader
    /// carries on: a plugin that asks what it may not is not fatal.
    #[tokio::test]
    async fn a_request_the_host_does_not_serve_is_refused_and_the_reader_goes_on() {
        let router = Router {
            alive: AtomicBool::new(true),
            ..Router::default()
        };
        router
            .line(
                "noisy",
                r#"{"jsonrpc":"2.0","id":1,"method":"tool/call","params":{}}"#,
            )
            .await;
        router
            .line(
                "noisy",
                r#"{"jsonrpc":"2.0","id":2,"method":"service/call","params":{"key":"kv","method":"get"}}"#,
            )
            .await;
        assert!(router.alive.load(Ordering::Acquire));
    }
}
