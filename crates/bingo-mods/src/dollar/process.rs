//! A plugin and session own each tree; no PID is accepted from JavaScript.
use super::{
    scope::{LIMIT, cap},
    world::{ProcessInput, ProcessOutput},
};
use bingo_sdk::{HOOK_DEADLINE, SessionId};
use std::{
    collections::BTreeMap,
    path::Path,
    process::Stdio,
    sync::{Arc, Mutex},
    time::Duration,
};
use tokio::{
    io::{AsyncRead, AsyncReadExt, AsyncWriteExt},
    process::{ChildStderr, ChildStdin, ChildStdout},
    sync::Mutex as AsyncMutex,
};
const GRACE: Duration = Duration::from_millis(100);
struct Held {
    owner: String,
    generation: bingo_sdk::CancellationToken,
    watcher: Mutex<Option<tokio::task::JoinHandle<()>>>,
    session: SessionId,
    tree: AsyncMutex<bingo_process::Tree>,
    stdin: AsyncMutex<Option<ChildStdin>>,
    stdout: AsyncMutex<Option<ChildStdout>>,
    stderr: AsyncMutex<Option<ChildStderr>>,
}
#[derive(Default)]
pub struct Processes(Mutex<BTreeMap<String, Arc<Held>>>);
impl Processes {
    pub fn spawn(
        &self,
        plugin: &str,
        input: &ProcessInput,
        cwd: &Path,
        env: BTreeMap<String, String>,
        generation: bingo_sdk::CancellationToken,
    ) -> Result<String, String> {
        let mut held = start(plugin, input, cwd, env)?;
        held.generation = generation.clone();
        let held = Arc::new(held);
        let weak = Arc::downgrade(&held);
        let task = tokio::spawn(async move {
            generation.cancelled().await;
            if let Some(held) = weak.upgrade() {
                let _ = held.tree.lock().await.terminate(GRACE).await;
            }
        });
        *held.watcher.lock().unwrap_or_else(|e| e.into_inner()) = Some(task);
        let id = bingo_sdk::ItemId::mint().to_string();
        self.0
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .insert(id.clone(), held);
        Ok(id)
    }
    fn get(&self, plugin: &str, session: &SessionId, id: &str) -> Result<Arc<Held>, String> {
        self.0
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .get(id)
            .filter(|held| {
                held.owner == plugin && held.session == *session && !held.generation.is_cancelled()
            })
            .cloned()
            .ok_or_else(|| "process handle is not owned by this plugin and session".into())
    }
    pub async fn write(
        &self,
        plugin: &str,
        session: &SessionId,
        id: &str,
        body: Option<String>,
    ) -> Result<(), String> {
        let held = self.get(plugin, session, id)?;
        let mut stdin = held.stdin.lock().await;
        match body {
            Some(body) => {
                cap(body.len(), LIMIT).map_err(failed)?;
                stdin
                    .as_mut()
                    .ok_or("process stdin is closed")?
                    .write_all(body.as_bytes())
                    .await
                    .map_err(failed)?;
            }
            None => {
                stdin.take();
            }
        }
        Ok(())
    }
    pub async fn read(
        &self,
        plugin: &str,
        session: &SessionId,
        id: &str,
    ) -> Result<ProcessOutput, String> {
        let held = self.get(plugin, session, id)?;
        let result = output(&held).await;
        if result.is_err() {
            self.kill(plugin, session, id).await?;
        }
        result
    }
    pub async fn kill(&self, plugin: &str, session: &SessionId, id: &str) -> Result<(), String> {
        let held = self.get(plugin, session, id)?;
        self.0.lock().unwrap_or_else(|e| e.into_inner()).remove(id);
        held.tree
            .lock()
            .await
            .terminate(GRACE)
            .await
            .map_err(failed)?;
        Ok(())
    }
    pub async fn remove(&self, plugin: Option<&str>, session: Option<&SessionId>) {
        let removed = {
            let mut map = self.0.lock().unwrap_or_else(|e| e.into_inner());
            let ids: Vec<_> = map
                .iter()
                .filter(|(_, held)| {
                    plugin.is_none_or(|p| held.owner == p)
                        && session.is_none_or(|s| held.session == *s)
                })
                .map(|(id, _)| id.clone())
                .collect();
            ids.into_iter()
                .filter_map(|id| map.remove(&id))
                .collect::<Vec<_>>()
        };
        for held in removed {
            let _ = held.tree.lock().await.terminate(GRACE).await;
        }
    }
}
pub async fn run(
    plugin: &str,
    input: ProcessInput,
    cwd: &Path,
    env: BTreeMap<String, String>,
) -> Result<ProcessOutput, String> {
    let timeout = input
        .timeout_ms
        .map(Duration::from_millis)
        .unwrap_or(HOOK_DEADLINE)
        .min(HOOK_DEADLINE);
    let held = start(plugin, &input, cwd, env)?;
    held.stdin.lock().await.take();
    tokio::time::timeout(timeout, output(&held))
        .await
        .map_err(|_| "process.run exceeded its timeout".to_owned())?
}
fn start(
    plugin: &str,
    input: &ProcessInput,
    cwd: &Path,
    env: BTreeMap<String, String>,
) -> Result<Held, String> {
    let (program, args) = input
        .argv
        .split_first()
        .ok_or("process argv must not be empty")?;
    if program.is_empty() {
        return Err("process program must not be empty".into());
    }
    cap(input.argv.iter().map(String::len).sum(), LIMIT).map_err(failed)?;
    let mut command = tokio::process::Command::new(program);
    command
        .args(args)
        .current_dir(cwd)
        .envs(env)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let mut tree = bingo_process::tree(command).map_err(failed)?;
    Ok(Held {
        generation: Default::default(),
        watcher: Default::default(),
        owner: plugin.into(),
        session: input.session.clone(),
        stdin: AsyncMutex::new(tree.take_stdin()),
        stdout: AsyncMutex::new(tree.take_stdout()),
        stderr: AsyncMutex::new(tree.take_stderr()),
        tree: AsyncMutex::new(tree),
    })
}
async fn output(held: &Held) -> Result<ProcessOutput, String> {
    let stdout = held
        .stdout
        .lock()
        .await
        .take()
        .ok_or("process stdout has already been read")?;
    let stderr = held
        .stderr
        .lock()
        .await
        .take()
        .ok_or("process stderr has already been read")?;
    let (stdout, stderr) = tokio::try_join!(capture(stdout), capture(stderr))?;
    let code = held.tree.lock().await.wait().await.map_err(failed)?.code();
    Ok(ProcessOutput {
        code,
        stdout,
        stderr,
    })
}
async fn capture(stream: impl AsyncRead + Unpin) -> Result<String, String> {
    let mut bytes = Vec::new();
    stream
        .take((LIMIT + 1) as u64)
        .read_to_end(&mut bytes)
        .await
        .map_err(failed)?;
    cap(bytes.len(), LIMIT).map_err(failed)?;
    Ok(String::from_utf8_lossy(&bytes).into_owned())
}
fn failed(error: impl std::fmt::Display) -> String {
    error.to_string()
}

impl Drop for Held {
    fn drop(&mut self) {
        if let Some(watcher) = self
            .watcher
            .get_mut()
            .unwrap_or_else(|e| e.into_inner())
            .take()
        {
            watcher.abort();
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[tokio::test]
    async fn generation_drop_closes_grandchild_stdout_and_foreign_ids_are_refused() {
        let home = tempfile::tempdir().unwrap();
        let processes = Processes::default();
        let generation = bingo_sdk::CancellationToken::new();
        let input = ProcessInput {
            session: SessionId::mint(),
            argv: vec![
                bingo_process::shell::resolve()
                    .unwrap()
                    .path()
                    .to_string_lossy()
                    .into_owned(),
                "-c".into(),
                "sleep 60 & echo ready; wait".into(),
            ],
            cwd: None,
            timeout_ms: None,
        };
        let id = processes
            .spawn(
                "owner",
                &input,
                home.path(),
                Default::default(),
                generation.clone(),
            )
            .unwrap();
        assert!(
            processes
                .kill("other", &input.session, &id)
                .await
                .unwrap_err()
                .contains("not owned")
        );
        let held = processes.get("owner", &input.session, &id).unwrap();
        let mut stdout = tokio::io::BufReader::new(held.stdout.lock().await.take().unwrap());
        let mut ready = String::new();
        use tokio::io::AsyncBufReadExt;
        tokio::time::timeout(Duration::from_secs(10), stdout.read_line(&mut ready))
            .await
            .unwrap()
            .unwrap();
        assert_eq!(ready, "ready\n");
        generation.cancel();
        let mut rest = Vec::new();
        tokio::time::timeout(Duration::from_secs(10), stdout.read_to_end(&mut rest))
            .await
            .expect("tree descendants close stdout")
            .unwrap();
        processes.remove(None, None).await;
    }
}
