#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::io::{BufRead, Read, Write};
use std::path::Path;
use std::process::{Child, Command, Stdio};
use std::time::Duration;

use bingo_gateway::{paths::Paths, probe::Probe, run, state::State};
use bingo_sdk::Env;

#[derive(Debug)]
struct Table(u32);
impl Probe for Table {
    fn alive(&self, pid: u32) -> bool {
        pid == self.0
    }
    fn command(&self, _: u32) -> Option<String> {
        Some("unrelated-child".into())
    }
    fn terminate(&self, _: u32) -> Result<(), String> {
        panic!("must never signal a reused pid")
    }
}

struct Owned(Child);
impl Drop for Owned {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

fn child(home: &Path, resident: bool, live: u32) -> (Owned, String) {
    let mut child = Command::new(std::env::current_exe().unwrap())
        .args(["--exact", "child_process", "--nocapture"])
        .env("M129_CHILD_HOME", home)
        .env("M129_CHILD_RESIDENT", resident.to_string())
        .env("M129_CHILD_LIVE", live.to_string())
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .unwrap();
    let stdout = child.stdout.take().unwrap();
    let owned = Owned(child);
    let (send, recv) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        let answer = std::io::BufReader::new(stdout)
            .lines()
            .map_while(Result::ok)
            .find(|line| line.starts_with("M129:"));
        let _ = send.send(answer);
    });
    let answer = recv.recv_timeout(Duration::from_secs(30)).unwrap().unwrap();
    (owned, answer)
}

#[test]
fn child_process() {
    let Some(home) = std::env::var_os("M129_CHILD_HOME") else {
        return;
    };
    let runtime = tokio::runtime::Runtime::new().unwrap();
    let _entered = runtime.enter();
    let paths = Paths::new(&Env::rooted(home));
    let resident = if std::env::var("M129_CHILD_RESIDENT").unwrap() == "true" {
        let live = std::env::var("M129_CHILD_LIVE").unwrap().parse().unwrap();
        match run::enter(&paths, &Table(live)) {
            Ok(resident) => Some(resident),
            Err(error) => {
                println!("M129:refused {error}");
                return;
            }
        }
    } else {
        None
    };
    println!("M129:ready");
    std::io::stdout().flush().unwrap();
    let _ = std::io::stdin().read(&mut [0]);
    drop(resident);
}

fn record(home: &Path, pid: u32, claimed: bool) -> Paths {
    let paths = Paths::new(&Env::rooted(home));
    paths.ensure().unwrap();
    let mut record = serde_json::json!({
        "pid": pid, "version": "0.6.6", "started": "2026-10-13T00:00:00Z"
    });
    if claimed {
        record["claimed"] = true.into();
    }
    std::fs::write(paths.pidfile(), record.to_string()).unwrap();
    paths
}

#[test]
fn a_claimed_record_naming_a_live_non_gateway_does_not_block_run() {
    let home = tempfile::tempdir().unwrap();
    let (other, _) = child(home.path(), false, 0);
    record(home.path(), other.0.id(), true);
    let (_gateway, answer) = child(home.path(), true, other.0.id());
    assert_eq!(answer, "M129:ready");
}

#[test]
fn a_killed_gateway_can_restart_without_removing_files() {
    let home = tempfile::tempdir().unwrap();
    let (mut first, answer) = child(home.path(), true, 0);
    assert_eq!(answer, "M129:ready");
    // `Child::wait` closes stdin; keep it open so only the kill can end the child.
    let _input = first.0.stdin.take().unwrap();
    first.0.kill().unwrap();
    first.0.wait().unwrap();
    let paths = Paths::new(&Env::rooted(home.path()));
    assert!(paths.pidfile().exists());
    let (_second, answer) = child(home.path(), true, 0);
    assert_eq!(answer, "M129:ready");
}

#[test]
fn a_legacy_record_naming_a_live_child_still_refuses() {
    let home = tempfile::tempdir().unwrap();
    let (other, _) = child(home.path(), false, 0);
    record(home.path(), other.0.id(), false);
    let (_gateway, answer) = child(home.path(), true, other.0.id());
    assert!(answer.contains("already holds this data dir"), "{answer}");
}

#[tokio::test]
async fn a_free_claim_is_stale_and_stop_never_signals_its_live_pid() {
    let home = tempfile::tempdir().unwrap();
    let (other, _) = child(home.path(), false, 0);
    let paths = record(home.path(), other.0.id(), true);
    let table = Table(other.0.id());
    assert!(matches!(
        State::read(&paths, &table).unwrap(),
        State::Stale(_)
    ));
    let status = bingo_gateway::status::status(&paths, home.path(), &table).unwrap();
    assert!(
        status.contains("no longer owns the gateway claim"),
        "{status}"
    );
    bingo_gateway::stop::stop(&paths, home.path(), &table)
        .await
        .unwrap();
}

#[test]
fn new_records_declare_claim_ownership() {
    let record = bingo_gateway::pidfile::Record::here(jiff::Timestamp::UNIX_EPOCH);
    let json: serde_json::Value =
        serde_json::from_str(&bingo_gateway::pidfile::render(&record)).unwrap();
    assert_eq!(json["claimed"], true);
}

#[test]
fn a_second_gateway_cannot_overwrite_a_held_claim() {
    let home = tempfile::tempdir().unwrap();
    let (first, answer) = child(home.path(), true, 0);
    assert_eq!(answer, "M129:ready");
    let (_second, answer) = child(home.path(), true, first.0.id());
    assert!(answer.starts_with("M129:refused"), "{answer}");
    let paths = Paths::new(&Env::rooted(home.path()));
    let record = bingo_gateway::pidfile::read(&paths.pidfile())
        .unwrap()
        .unwrap();
    assert_eq!(record.pid, first.0.id());
}

#[test]
fn doctor_never_removes_the_gateway_claim_file() {
    let home = tempfile::tempdir().unwrap();
    let env = Env::rooted(home.path());
    let paths = Paths::new(&env);
    let patient = bingo_gateway::doctor::Patient {
        paths: &paths,
        env: &env,
        cwd: home.path(),
        settings: None,
    };
    let claim =
        bingo_claim::Claim::take(paths.data_dir(), bingo_gateway::pidfile::PROTOCOL).unwrap();
    let path = claim.path().to_owned();
    let held = bingo_gateway::doctor::doctor(&patient, &Table(0), true).unwrap();
    assert!(held.contains("gateway OS lock is held"), "{held}");
    assert!(path.exists());
    drop(claim);
    let free = bingo_gateway::doctor::doctor(&patient, &Table(0), true).unwrap();
    assert!(free.contains("gateway OS lock is free"), "{free}");
    assert_eq!(std::fs::read_to_string(path).unwrap(), "bingo-gateway-v1\n");
}
