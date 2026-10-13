//! Test process output with tree ownership.
#![allow(clippy::unwrap_used)]
use std::process::Stdio;
use tokio::process::Command;

/// Collect a tree's output while retaining drop ownership through cancellation.
pub async fn output(mut command: Command) -> std::io::Result<std::process::Output> {
    use tokio::io::AsyncReadExt;
    command
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let mut child = bingo_process::tree(command).map_err(|error| error.0)?;
    let mut stdout = child.take_stdout().unwrap();
    let mut stderr = child.take_stderr().unwrap();
    let (mut out, mut err) = (Vec::new(), Vec::new());
    let (status, _, _) = tokio::try_join!(
        child.wait(),
        stdout.read_to_end(&mut out),
        stderr.read_to_end(&mut err)
    )?;
    Ok(std::process::Output {
        status,
        stdout: out,
        stderr: err,
    })
}
