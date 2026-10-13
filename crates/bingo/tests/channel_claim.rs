//! Ownership survives a killed channel host without manual file cleanup.
#![allow(clippy::unwrap_used, clippy::expect_used)]
mod support;

use std::process::Stdio;
use std::time::Duration;
use tokio::net::TcpListener;
use tokio::process::Command;

#[tokio::test]
async fn a_live_channel_host_excludes_another_and_a_killed_one_releases() {
    let home = tempfile::tempdir().unwrap();
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap().to_string();
    let command = || {
        let mut command = Command::new(env!("CARGO_BIN_EXE_bingo"));
        command
            .args(["channels", "--cwd"])
            .arg(home.path())
            .args(["--channels", &format!("loopback={address}")])
            .envs(support::home::home_env(home.path()))
            .env(bingo_channel_adapter::loopback::ENABLE_ENV, "1")
            .env_remove("BINGO_FAKE_SCRIPT")
            .env_remove("OPENAI_API_KEY")
            .env_remove("ANTHROPIC_API_KEY")
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        command
    };
    let limit = Duration::from_secs(30);
    let mut first = bingo_process::tree(command()).unwrap();
    let (_peer, _) = tokio::time::timeout(limit, listener.accept())
        .await
        .unwrap()
        .unwrap();
    let second = tokio::time::timeout(limit, support::process::output(command()))
        .await
        .unwrap()
        .unwrap();
    assert!(!second.status.success());
    assert!(second.stdout.is_empty());
    let stderr = String::from_utf8_lossy(&second.stderr);
    assert!(stderr.contains("another bingo already runs"), "{stderr}");
    first.start_kill().unwrap();
    first.wait().await.unwrap();
    let locks = home.path().join(".bingo/data/channels");
    let path = std::fs::read_dir(locks)
        .unwrap()
        .next()
        .unwrap()
        .unwrap()
        .path();
    let marker = std::fs::read(&path).unwrap();
    let mut next = bingo_process::tree(command()).unwrap();
    let connected = tokio::select! {
        connection = listener.accept() => connection.is_ok(),
        status = next.wait() => panic!("successor exited instead of taking the claim: {status:?}"),
        () = tokio::time::sleep(limit) => panic!("successor never connected"),
    };
    assert!(connected);
    next.start_kill().unwrap();
    next.wait().await.unwrap();
    assert_eq!(marker, b"bingo-channel-claim-v1\n");
    assert_eq!(std::fs::read(path).unwrap(), marker);
}
