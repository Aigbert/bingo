use std::process::Stdio;
use std::time::Duration;
use tokio::io::{AsyncBufReadExt, AsyncReadExt, BufReader};

#[tokio::test]
async fn dropping_a_tree_closes_a_grandchilds_stdout() {
    let shell = crate::shell::resolve().expect("a POSIX shell");
    let mut command = tokio::process::Command::new(shell.path());
    command.args(["-c", "sleep 60 & echo ready; wait"])
        .stdin(Stdio::null()).stdout(Stdio::piped()).stderr(Stdio::null());
    // Baseline: killing only the shell leaves its grandchild holding stdout.
    let mut child = command.kill_on_drop(true).spawn().expect("spawn");
    let mut stdout = BufReader::new(child.stdout.take().expect("stdout"));
    let mut ready = String::new();
    tokio::time::timeout(Duration::from_secs(10), stdout.read_line(&mut ready))
        .await.expect("shell ready").expect("read");
    assert_eq!(ready, "ready\n");
    drop(child);
    let mut rest = Vec::new();
    tokio::time::timeout(Duration::from_secs(3), stdout.read_to_end(&mut rest))
        .await.expect("the grandchild must close stdout when its tree drops")
        .expect("EOF");
}
