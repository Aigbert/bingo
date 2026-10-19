//! The process a command runs in.
//!
//! The shell is spawned as its own process-group leader, so a timeout or an
//! interrupt reaches every grandchild instead of orphaning the ones the shell
//! started. Its stdin is `/dev/null`, so nothing it runs can wait for a person.
//! Its stdout and stderr go into one sink in arrival order, because a
//! command's error lines are part of its story and the model reads them where
//! they happened.
//!
//! A run comes apart into three: [`start`] spawns and begins draining the
//! pipes, [`watch`] waits for whichever of the exit, the deadline, the
//! interrupt or a promotion comes first, and [`conclude`] turns what is left
//! into an answer. A promotion keeps the [`Running`] instead of concluding it:
//! the same process and the same pipes go to the job table (ADR-0018 §6).
//!
//! There is a fourth way a run ends, and it writes no answer at all: a person
//! stops the turn, the kernel drops the call's future, and [`Tree`] takes the
//! process tree down on its way out. Nothing is waited for and the output is
//! forfeit — a background job is not a call and is never ended this way.
//!
//! Shell resolution is shared with shell hooks through `bingo-process`.

use std::path::Path;
use std::process::{ExitStatus, Stdio};
use std::sync::Arc;
use std::time::Duration;

use bingo_process::Tree;
use bingo_sdk::{CancellationToken, ToolContext, ToolError};
use tokio::io::{AsyncRead, AsyncReadExt};
use tokio::process::Command;
use tokio::sync::Mutex;
use tokio::task::JoinHandle;

use crate::output::{Ended, MAX_OUTPUT_CHARS};
use crate::sink::Sink;
use crate::tail::{self, Progress, Tail, ToCall, Unwatched};

/// How long the pipes are given once the process is gone. A grandchild that
/// inherited them and outlived the kill must not hold the turn open.
const DRAIN: Duration = Duration::from_secs(2);

/// What a command left behind: everything it wrote, within the cap, and the way
/// it ended.
#[derive(Debug)]
pub struct Run {
    pub output: String,
    /// Whether `output` is everything the command wrote, nothing cut from its
    /// middle.
    pub whole: bool,
    pub ended: Ended,
}

/// A command under way: the process group, the tasks draining its pipes, and
/// where its output is going. All three move together, which is what lets a
/// running command change hands without restarting.
pub struct Running {
    pub child: Tree,
    pub readers: Vec<Reader>,
    pub sink: Arc<Mutex<Sink>>,
}

/// What running a command takes from whoever asked for it: the directory it
/// starts in, the flag that hands it to the background, and where its tail
/// goes while it works. A tool call lends all three; a line a person typed
/// lends only the first.
///
/// The turn's interrupt is not among them and needs no token here: it is the
/// call's future being dropped, which takes [`Tree`] and the process tree
/// with it. A second reading of the same interrupt would be a second answer
/// to one keypress.
pub struct Context<'a> {
    cwd: &'a Path,
    promote: CancellationToken,
    progress: Box<dyn Progress + 'a>,
}

impl<'a> Context<'a> {
    /// A tool call: the call's own progress line is where its tail goes, and
    /// `promote` is the flag a surface flips to take it into the background.
    pub fn of_call(cx: &'a ToolContext, promote: CancellationToken) -> Self {
        Self {
            cwd: &cx.cwd,
            promote,
            progress: Box::new(ToCall(cx)),
        }
    }

    /// A line a person typed: nothing but the timeout stops it, no call is
    /// watching it, and there is no call id to promote it by.
    pub fn unwatched(cwd: &'a Path) -> Self {
        Self {
            cwd,
            promote: CancellationToken::new(),
            progress: Box::new(Unwatched),
        }
    }
}

/// Run one command to its end, to the timeout, or to the interrupt.
pub async fn run(command: &str, timeout: Duration, cx: &Context<'_>) -> Result<Run, ToolError> {
    let mut running = start(command, cx.cwd, Sink::buffer(MAX_OUTPUT_CHARS))?;
    let over = match watch(&mut running, timeout, cx).await? {
        Stop::Over(over) => over,
        // Nobody was handed this run's promote token, so this cannot happen;
        // killing the group and reporting what it wrote is the safe reading.
        Stop::Promoted => Over::Interrupted,
    };
    conclude(running, over, timeout).await
}

/// Spawn a command and start draining its pipes into `sink`.
pub fn start(command: &str, cwd: &Path, sink: Sink) -> Result<Running, ToolError> {
    let mut child = spawn(command, cwd)?;
    let sink = Arc::new(Mutex::new(sink));
    let readers = read_pipes(&mut child, &sink);
    Ok(Running {
        child,
        readers,
        sink,
    })
}

fn spawn(command: &str, cwd: &Path) -> Result<Tree, ToolError> {
    let shell =
        bingo_process::shell::resolve().map_err(|error| ToolError::Failed(error.to_string()))?;
    let mut shell_command = Command::new(shell.path());
    shell_command
        .arg("-c")
        .arg(command)
        .current_dir(cwd)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    bingo_process::tree(shell_command)
        .map_err(|e| ToolError::Failed(format!("could not run {}: {e}", shell.path().display())))
}

/// How the wait ended.
pub enum Stop {
    /// The command is nobody's to wait for any more.
    Over(Over),
    /// A person took it into the background: it is still running.
    Promoted,
}

/// The three ways a wait ends with the command over.
pub enum Over {
    Exited(ExitStatus),
    Timeout,
    Interrupted,
}

/// Wait for the command, sampling the tail on the way, and stop for whichever
/// comes first: the exit, the deadline, or the promotion. There is no fourth
/// branch for the turn's interrupt: an interrupt drops this future, and this
/// function never gets to return at all.
pub async fn watch(
    running: &mut Running,
    timeout: Duration,
    cx: &Context<'_>,
) -> Result<Stop, ToolError> {
    let deadline = tokio::time::Instant::now() + timeout;
    let mut tail = Tail::default();
    // The two fields are borrowed apart, so waiting on the process and
    // sampling what it wrote do not contend for the whole of `running`.
    let child = &mut running.child;
    let sink = &running.sink;
    loop {
        let over = tokio::select! {
            status = child.wait() => Over::Exited(
                status.map_err(|e| ToolError::Failed(format!("waiting for the command: {e}")))?,
            ),
            () = tokio::time::sleep_until(deadline) => Over::Timeout,
            () = cx.promote.cancelled() => return Ok(Stop::Promoted),
            () = tokio::time::sleep(tail::INTERVAL) => {
                tail.sample(sink, cx.progress.as_ref()).await;
                continue;
            }
        };
        return Ok(Stop::Over(over));
    }
}

/// An exit needs nothing; anything else kills the group first. The kernel keeps
/// an interrupted `Block` tool's real output, so an interrupt is reported as
/// what the command had produced, not as a cancellation.
pub async fn conclude(
    mut running: Running,
    over: Over,
    timeout: Duration,
) -> Result<Run, ToolError> {
    let ended = match over {
        // A shell killed by a signal left no status of its own to report.
        Over::Exited(status) => Ended::Exited(status.code().unwrap_or(-1)),
        Over::Timeout => {
            kill(&mut running.child).await;
            Ended::Timeout {
                after_ms: timeout.as_millis().try_into().unwrap_or(u64::MAX),
            }
        }
        Over::Interrupted => {
            kill(&mut running.child).await;
            Ended::Interrupted
        }
    };
    drain(running.readers).await;
    let sink = running.sink.lock().await;
    let (output, whole) = (sink.finish(), sink.whole());
    Ok(Run {
        output,
        whole,
        ended,
    })
}

/// `SIGKILL` to the whole group — the wrapper's `start_kill` is a `killpg`, so a
/// grandchild that outlived its parent goes with it — then reap what is left,
/// bounded so an escapee cannot hold the turn.
pub async fn kill(child: &mut Tree) {
    let _ = child.start_kill();
    let _ = tokio::time::timeout(DRAIN, child.wait()).await;
}

pub type Reader = JoinHandle<()>;

/// One task per pipe, both writing into the same sink, so stdout and stderr
/// interleave in the order they arrived.
fn read_pipes(child: &mut Tree, sink: &Arc<Mutex<Sink>>) -> Vec<Reader> {
    let pipes = [
        child.take_stdout().map(boxed),
        child.take_stderr().map(boxed),
    ];
    pipes
        .into_iter()
        .flatten()
        .map(|pipe| tokio::spawn(pump(pipe, sink.clone())))
        .collect()
}

type Pipe = Box<dyn AsyncRead + Unpin + Send>;

fn boxed(pipe: impl AsyncRead + Unpin + Send + 'static) -> Pipe {
    Box::new(pipe)
}

/// Read one pipe to its end, decoding as the bytes arrive.
async fn pump(mut pipe: Pipe, sink: Arc<Mutex<Sink>>) {
    let mut buffer = [0u8; 8 * 1024];
    let mut stream = Utf8Stream::default();
    loop {
        let read = match pipe.read(&mut buffer).await {
            Ok(0) | Err(_) => return,
            Ok(read) => read,
        };
        let text = stream.decode(&buffer[..read]);
        if !text.is_empty() {
            sink.lock().await.push(&text).await;
        }
    }
}

/// Bytes arriving in whatever chunks the pipe hands over, out as text. A
/// character split across two reads waits for the second one instead of
/// becoming two replacements.
#[derive(Debug, Default)]
struct Utf8Stream {
    pending: Vec<u8>,
}

impl Utf8Stream {
    fn decode(&mut self, bytes: &[u8]) -> String {
        self.pending.extend_from_slice(bytes);
        let mut text = String::new();
        loop {
            let error = match std::str::from_utf8(&self.pending) {
                Ok(all) => {
                    text.push_str(all);
                    self.pending.clear();
                    return text;
                }
                Err(error) => error,
            };
            let valid = error.valid_up_to();
            text.push_str(&String::from_utf8_lossy(&self.pending[..valid]));
            let Some(broken) = error.error_len() else {
                // The chunk ended mid-character: keep it for the next read.
                self.pending.drain(..valid);
                return text;
            };
            text.push(char::REPLACEMENT_CHARACTER);
            self.pending.drain(..valid + broken);
        }
    }
}

/// Let the readers finish what the pipes already hold, then let them go.
pub async fn drain(readers: Vec<Reader>) {
    let deadline = tokio::time::Instant::now() + DRAIN;
    for mut reader in readers {
        if tokio::time::timeout_at(deadline, &mut reader)
            .await
            .is_err()
        {
            reader.abort();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::{context, context_in};

    use std::path::PathBuf;

    const NO_TIMEOUT: Duration = Duration::from_secs(30);

    fn watched(cx: &ToolContext) -> Context<'_> {
        Context::of_call(cx, CancellationToken::new())
    }

    async fn bash(command: &str) -> Run {
        let (_host, cx) = context();
        run(command, NO_TIMEOUT, &watched(&cx))
            .await
            .expect("the command ran")
    }

    #[tokio::test]
    async fn a_command_comes_back_with_what_it_wrote() {
        let out = bash("echo hi").await;
        assert_eq!(out.output, "hi\n");
        assert_eq!(out.ended, Ended::Exited(0));
    }

    #[tokio::test]
    async fn stderr_arrives_with_stdout() {
        let out = bash("echo out; echo err >&2").await;
        assert!(out.output.contains("out"), "{:?}", out.output);
        assert!(out.output.contains("err"), "{:?}", out.output);
    }

    #[tokio::test]
    async fn the_command_runs_in_the_call_s_working_directory() {
        let dir = tempfile::tempdir().expect("temp dir");
        std::fs::write(dir.path().join("marker"), "").expect("write");
        let (_host, cx) = context_in(dir.path().to_path_buf());
        let out = run("ls", NO_TIMEOUT, &watched(&cx))
            .await
            .expect("the command ran");
        assert_eq!(out.output, "marker\n");
    }

    #[tokio::test]
    async fn a_command_nobody_watches_still_runs_where_it_was_told() {
        let dir = tempfile::tempdir().expect("temp dir");
        std::fs::write(dir.path().join("marker"), "").expect("write");
        let out = run("ls", NO_TIMEOUT, &Context::unwatched(dir.path()))
            .await
            .expect("the command ran");
        assert_eq!(out.output, "marker\n");
    }

    #[tokio::test]
    async fn a_directory_that_is_not_there_fails_to_spawn() {
        let (_host, cx) = context_in(PathBuf::from("/no/such/directory/here"));
        let error = run("echo hi", NO_TIMEOUT, &watched(&cx)).await.err();
        assert!(
            matches!(&error, Some(ToolError::Failed(m)) if m.starts_with("could not run")),
            "got {error:?}"
        );
    }

    #[tokio::test]
    async fn multibyte_output_survives_the_pipe() {
        // 60 000 bytes: several reads, and the character on every boundary.
        let out = bash("printf '字%.0s' $(seq 1 20000)").await;
        assert_eq!(out.output.chars().filter(|c| *c == '字').count(), 20_000);
        assert!(!out.output.contains(char::REPLACEMENT_CHARACTER));
    }

    #[tokio::test]
    async fn the_timeout_kills_the_whole_group_and_says_so() {
        let dir = tempfile::tempdir().expect("temp dir");
        let ticks = dir.path().join("ticks");
        let (_host, cx) = context();
        let command = format!(
            "(while true; do echo tick >> '{0}'; sleep 0.02; done) & sleep 30",
            ticks.display()
        );

        // Long enough for the shell to start its grandchild on a loaded
        // Windows runner, where Git Bash takes hundreds of milliseconds to fork.
        let limit = Duration::from_secs(2);
        let out =
            tokio::time::timeout(Duration::from_secs(20), run(&command, limit, &watched(&cx)))
                .await
                .expect("the shell timeout did not finish")
                .expect("the command ran");

        assert_eq!(out.ended, Ended::Timeout { after_ms: 2000 });

        // The grandchild was in the group, so it stopped writing with the shell.
        let after_kill = std::fs::metadata(&ticks).map(|m| m.len()).unwrap_or(0);
        tokio::time::sleep(Duration::from_millis(300)).await;
        let later = std::fs::metadata(&ticks).map(|m| m.len()).unwrap_or(0);
        assert!(after_kill > 0, "the grandchild never ran");
        assert_eq!(after_kill, later, "the grandchild outlived the group");
    }

    #[tokio::test]
    async fn output_past_the_cap_is_truncated_in_the_middle() {
        let out = bash("yes | head -c 200000").await;
        assert_eq!(out.ended, Ended::Exited(0));
        assert!(out.output.contains("chars truncated"), "no marker");
        assert!(
            out.output.chars().count() < MAX_OUTPUT_CHARS + 100,
            "{} characters",
            out.output.chars().count()
        );
        assert!(out.output.starts_with("y\ny\n"), "the head is missing");
        assert!(out.output.ends_with("y\n"), "the tail is missing");
    }

    /// A person stopping the turn drops the call's future. Nothing here is
    /// asked and nothing is waited for: the whole tree goes with the drop,
    /// the grandchild the shell backgrounded included.
    #[tokio::test]
    async fn letting_go_of_a_run_takes_the_whole_process_group_with_it() {
        let dir = tempfile::tempdir().expect("temp dir");
        let ticks = dir.path().join("ticks");
        let (_host, cx) = context_in(dir.path().to_path_buf());
        let command = format!(
            "(while true; do echo tick >> '{0}'; sleep 0.05; done) & sleep 30",
            ticks.display()
        );
        let context = watched(&cx);
        let mut running = Box::pin(run(&command, NO_TIMEOUT, &context));
        // One poll spawns it; then wait for the grandchild to be at work
        // rather than for a clock, because a loaded box makes a guess of any
        // fixed wait and this test's subject is a command that is definitely
        // running.
        let polled = tokio::time::timeout(Duration::from_millis(50), &mut running).await;
        assert!(polled.is_err(), "the command outlives its first poll");
        let wrote = wait_for_output(&ticks).await;
        assert!(wrote > 0, "the grandchild never ran");

        drop(running);
        tokio::time::sleep(SETTLE).await;
        let after = size(&ticks);
        tokio::time::sleep(SETTLE).await;
        assert_eq!(size(&ticks), after, "the process group outlived the drop");
    }

    /// Long enough that a killed group has certainly stopped writing, short
    /// enough to run twice in a test. It bounds nothing that must be waited
    /// for — only how long the proof watches for a writer that should be gone.
    const SETTLE: Duration = Duration::from_millis(300);

    fn size(path: &Path) -> u64 {
        std::fs::metadata(path).map(|m| m.len()).unwrap_or(0)
    }

    /// Poll until the file has something in it, bounded generously.
    async fn wait_for_output(path: &Path) -> u64 {
        for _ in 0..200 {
            tokio::time::sleep(Duration::from_millis(20)).await;
            let written = size(path);
            if written > 0 {
                return written;
            }
        }
        0
    }

    /// The seam promotion stands on: the wait ends, and the process, its pipes
    /// and its buffer are still there to be handed on.
    #[tokio::test]
    async fn a_promoted_wait_hands_the_running_command_back_untouched() {
        let (_host, cx) = context();
        let promote = CancellationToken::new();
        let flag = promote.clone();
        tokio::spawn(async move {
            tokio::time::sleep(Duration::from_millis(150)).await;
            flag.cancel();
        });

        let mut running = start(
            "echo started; sleep 30",
            &cx.cwd,
            Sink::buffer(MAX_OUTPUT_CHARS),
        )
        .expect("spawned");
        let stop = watch(
            &mut running,
            NO_TIMEOUT,
            &Context::of_call(&cx, promote.clone()),
        )
        .await
        .expect("the wait ended");
        assert!(matches!(stop, Stop::Promoted));
        assert!(running.child.id().is_some(), "the process is still there");
        assert_eq!(running.sink.lock().await.finish(), "started\n");

        let out = conclude(running, Over::Interrupted, NO_TIMEOUT)
            .await
            .expect("concluded");
        assert_eq!(out.output, "started\n");
    }

    #[test]
    fn a_character_split_across_two_reads_is_decoded_once() {
        let mut stream = Utf8Stream::default();
        let bytes = "字".as_bytes();
        assert_eq!(stream.decode(&bytes[..1]), "");
        assert_eq!(stream.decode(&bytes[1..]), "字");
    }

    #[test]
    fn bytes_that_are_not_text_become_one_replacement_each() {
        let mut stream = Utf8Stream::default();
        assert_eq!(stream.decode(b"a\xffb"), "a\u{fffd}b");
    }
}
